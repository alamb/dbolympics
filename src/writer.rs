//! Validating Parquet writer for one table.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use arrow::array::{
    ArrayRef, BooleanBuilder, Date32Builder, Decimal128Builder, Float64Builder, Int16Builder, Int32Builder,
    Int64Builder, ListBuilder, StringBuilder, TimestampMicrosecondBuilder,
};
use arrow::datatypes::{DataType, Field, Schema as ArrowSchema, SchemaRef, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;

use crate::rng::format_uuid;
use crate::schema::{Table, Type};
use crate::value::{Row, V};

const BATCH_ROWS: usize = 8192;
const ROW_GROUP_ROWS: usize = 122_880;

fn arrow_type(t: &Type) -> DataType {
    match t {
        Type::BigInt => DataType::Int64,
        Type::Integer => DataType::Int32,
        Type::SmallInt => DataType::Int16,
        Type::Boolean => DataType::Boolean,
        Type::Decimal { precision, scale } => DataType::Decimal128(*precision, *scale),
        Type::Double => DataType::Float64,
        Type::Varchar(_) | Type::Uuid | Type::Json => DataType::Utf8,
        Type::Date => DataType::Date32,
        Type::Timestamp => DataType::Timestamp(TimeUnit::Microsecond, None),
        Type::Array(e) => DataType::List(Arc::new(Field::new("item", arrow_type(e), true))),
    }
}

pub fn arrow_schema(t: &Table) -> SchemaRef {
    Arc::new(ArrowSchema::new(
        t.columns.iter().map(|c| Field::new(&c.name, arrow_type(&c.ty), c.nullable)).collect::<Vec<_>>(),
    ))
}

enum Col {
    I64(Int64Builder),
    I32(Int32Builder),
    I16(Int16Builder),
    Bool(BooleanBuilder),
    Dec(Decimal128Builder, u8, i8),
    F64(Float64Builder),
    Str(StringBuilder, Option<usize>),
    Date(Date32Builder),
    Ts(TimestampMicrosecondBuilder),
    List(ListBuilder<StringBuilder>, usize),
}

fn new_col(t: &Type) -> Col {
    match t {
        Type::BigInt => Col::I64(Int64Builder::new()),
        Type::Integer => Col::I32(Int32Builder::new()),
        Type::SmallInt => Col::I16(Int16Builder::new()),
        Type::Boolean => Col::Bool(BooleanBuilder::new()),
        Type::Decimal { precision, scale } => Col::Dec(
            Decimal128Builder::new().with_precision_and_scale(*precision, *scale).unwrap(),
            *precision,
            *scale,
        ),
        Type::Double => Col::F64(Float64Builder::new()),
        Type::Varchar(n) => Col::Str(StringBuilder::new(), Some(*n)),
        Type::Uuid | Type::Json => Col::Str(StringBuilder::new(), None),
        Type::Date => Col::Date(Date32Builder::new()),
        Type::Timestamp => Col::Ts(TimestampMicrosecondBuilder::new()),
        Type::Array(e) => match &**e {
            Type::Varchar(n) => Col::List(ListBuilder::new(StringBuilder::new()), *n),
            other => panic!("unsupported array element {other:?}"),
        },
    }
}

fn pow10(e: i32) -> i128 {
    10i128.pow(e as u32)
}

/// Writes one Parquet file of a table: `<out>/<table>/<name>.parquet`.
pub struct TableWriter {
    table: &'static Table,
    path: PathBuf,
    schema: SchemaRef,
    cols: Vec<Col>,
    batch_rows: usize,
    writer: Option<ArrowWriter<File>>,
    pub total_rows: u64,
}

impl TableWriter {
    pub fn new(table: &'static Table, out: &Path, name: &str) -> TableWriter {
        TableWriter {
            table,
            path: out.join(&table.name).join(format!("{name}.parquet")),
            schema: arrow_schema(table),
            cols: table.columns.iter().map(|c| new_col(&c.ty)).collect(),
            batch_rows: 0,
            writer: None,
            total_rows: 0,
        }
    }

    pub fn table_name(&self) -> &str {
        &self.table.name
    }

    /// Append one fully resolved row (no pending refs), validating it against the schema.
    pub fn push(&mut self, row: &Row) -> Result<()> {
        let t = self.table;
        for (i, (v, col)) in row.vals.iter().zip(self.cols.iter_mut()).enumerate() {
            let c = &t.columns[i];
            if v.is_null() {
                if !c.nullable {
                    bail!("{}.{}: NULL in NOT NULL column", t.name, c.name);
                }
                append_null(col);
                continue;
            }
            append(col, v).with_context(|| format!("{}.{} = {:?}", t.name, c.name, v))?;
        }
        self.batch_rows += 1;
        self.total_rows += 1;
        if self.batch_rows >= BATCH_ROWS {
            self.flush_batch()?;
        }
        Ok(())
    }

    fn flush_batch(&mut self) -> Result<()> {
        if self.batch_rows == 0 {
            return Ok(());
        }
        let arrays: Vec<ArrayRef> = self.cols.iter_mut().map(finish).collect();
        let batch = RecordBatch::try_new(self.schema.clone(), arrays)?;
        if self.writer.is_none() {
            self.open()?;
        }
        self.writer.as_mut().unwrap().write(&batch)?;
        self.batch_rows = 0;
        Ok(())
    }

    fn open(&mut self) -> Result<()> {
        let path = &self.path;
        fs::create_dir_all(path.parent().unwrap())?;
        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(ZstdLevel::try_new(3)?))
            .set_max_row_group_row_count(Some(ROW_GROUP_ROWS))
            .build();
        let f = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        self.writer = Some(ArrowWriter::try_new(f, self.schema.clone(), Some(props))?);
        Ok(())
    }

    /// Flush and close; writes an empty file if it never got a row.
    pub fn finish(&mut self) -> Result<()> {
        self.flush_batch()?;
        if self.writer.is_none() && self.total_rows == 0 {
            self.open()?;
        }
        if let Some(w) = self.writer.take() {
            w.close()?;
        }
        Ok(())
    }
}

fn append_null(col: &mut Col) {
    match col {
        Col::I64(b) => b.append_null(),
        Col::I32(b) => b.append_null(),
        Col::I16(b) => b.append_null(),
        Col::Bool(b) => b.append_null(),
        Col::Dec(b, ..) => b.append_null(),
        Col::F64(b) => b.append_null(),
        Col::Str(b, _) => b.append_null(),
        Col::Date(b) => b.append_null(),
        Col::Ts(b) => b.append_null(),
        Col::List(b, _) => b.append_null(),
    }
}

fn int_of(v: &V) -> Result<i64> {
    match v {
        V::Int(i) => Ok(*i),
        V::Ref(r) if !r.is_pending() => Ok(r.raw_id()),
        V::Ref(_) => bail!("unresolved reference"),
        _ => bail!("expected an integer"),
    }
}

fn str_len_ok(s: &str, max: Option<usize>) -> Result<()> {
    if let Some(n) = max {
        let len = s.chars().count();
        if len > n {
            bail!("string of length {len} exceeds varchar({n})");
        }
    }
    Ok(())
}

fn append(col: &mut Col, v: &V) -> Result<()> {
    match col {
        Col::I64(b) => b.append_value(int_of(v)?),
        Col::I32(b) => b.append_value(i32::try_from(int_of(v)?)?),
        Col::I16(b) => b.append_value(i16::try_from(int_of(v)?)?),
        Col::Bool(b) => match v {
            V::Bool(x) => b.append_value(*x),
            _ => bail!("expected a boolean"),
        },
        Col::Dec(b, precision, scale) => {
            let (unscaled, s) = match v {
                V::Money(c) => (*c as i128, 2i8),
                V::Dec(u, s) => (*u, *s),
                V::Int(i) => (*i as i128, 0),
                V::F64(f) => (libm::round(*f * 1e6) as i128, 6),
                _ => bail!("expected a decimal"),
            };
            let val = if *scale >= s {
                unscaled * pow10((*scale - s) as i32)
            } else {
                let d = pow10((s - *scale) as i32);
                let q = unscaled / d;
                let r = unscaled % d;
                if 2 * r.abs() >= d { q + unscaled.signum() } else { q }
            };
            if val.abs() >= pow10(*precision as i32) {
                bail!("value exceeds decimal({precision},{scale})");
            }
            b.append_value(val)
        }
        Col::F64(b) => match v {
            V::F64(x) => b.append_value(*x),
            V::Int(i) => b.append_value(*i as f64),
            _ => bail!("expected a double"),
        },
        Col::Str(b, max) => match v {
            V::Str(s) => {
                str_len_ok(s, *max)?;
                b.append_value(s)
            }
            V::Static(s) => {
                str_len_ok(s, *max)?;
                b.append_value(s)
            }
            V::Json(s) => b.append_value(s),
            V::Uuid(u) => b.append_value(format_uuid(*u)),
            _ => bail!("expected a string"),
        },
        Col::Date(b) => match v {
            V::Date(d) => b.append_value(*d),
            _ => bail!("expected a date"),
        },
        Col::Ts(b) => match v {
            V::Ts(t) => b.append_value(*t),
            _ => bail!("expected a timestamp"),
        },
        Col::List(b, max) => match v {
            V::Arr(xs) => {
                for x in xs {
                    str_len_ok(x, Some(*max))?;
                    b.values().append_value(x);
                }
                b.append(true)
            }
            _ => bail!("expected an array"),
        },
    }
    Ok(())
}

fn finish(col: &mut Col) -> ArrayRef {
    match col {
        Col::I64(b) => Arc::new(b.finish()),
        Col::I32(b) => Arc::new(b.finish()),
        Col::I16(b) => Arc::new(b.finish()),
        Col::Bool(b) => Arc::new(b.finish()),
        Col::Dec(b, ..) => Arc::new(b.finish()),
        Col::F64(b) => Arc::new(b.finish()),
        Col::Str(b, _) => Arc::new(b.finish()),
        Col::Date(b) => Arc::new(b.finish()),
        Col::Ts(b) => Arc::new(b.finish()),
        Col::List(b, _) => Arc::new(b.finish()),
    }
}
