//! Parquet copy of a generated dataset with array columns as PostgreSQL array literals in text
//! (`{"a","b"}`), for systems that do not read nested Parquet types but parse such literals into
//! their ARRAY columns on load (e.g. Umbra). Same directory layout as the input; every other
//! column is copied as it is. A system picks this copy with `input = "parquet-pgarrays"`.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use arrow::array::{Array, ArrayRef, AsArray, ListArray, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::{RecordBatch, RecordBatchReader};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;
use rayon::prelude::*;

/// A PostgreSQL array literal: every element double-quoted, `\` and `"` escaped, NULL unquoted.
pub fn literal(items: &dyn Array) -> String {
    let items = items.as_string::<i32>();
    let mut lit = String::with_capacity(16 * items.len() + 2);
    lit.push('{');
    for i in 0..items.len() {
        if i > 0 {
            lit.push(',');
        }
        if items.is_null(i) {
            lit.push_str("NULL");
            continue;
        }
        lit.push('"');
        for c in items.value(i).chars() {
            if c == '"' || c == '\\' {
                lit.push('\\');
            }
            lit.push(c);
        }
        lit.push('"');
    }
    lit.push('}');
    lit
}

fn convert_column(c: &ArrayRef) -> ArrayRef {
    let Some(l) = c.as_any().downcast_ref::<ListArray>() else { return c.clone() };
    let mut b = StringBuilder::with_capacity(l.len(), 32 * l.len());
    for r in 0..l.len() {
        if l.is_null(r) {
            b.append_null();
        } else {
            b.append_value(literal(l.value(r).as_ref()));
        }
    }
    Arc::new(b.finish())
}

fn file(src: &Path, dst: &Path) -> Result<()> {
    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(src)?)?.with_batch_size(8192).build()?;
    let schema = reader.schema();
    let fields: Vec<Field> = schema
        .fields()
        .iter()
        .map(|f| match f.data_type() {
            DataType::List(_) => Field::new(f.name(), DataType::Utf8, f.is_nullable()),
            _ => f.as_ref().clone(),
        })
        .collect();
    let out_schema = Arc::new(Schema::new(fields));
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::try_new(3)?))
        .set_max_row_group_row_count(Some(122_880))
        .build();
    let mut w = ArrowWriter::try_new(File::create(dst).with_context(|| format!("creating {}", dst.display()))?, out_schema.clone(), Some(props))?;
    for batch in reader {
        let batch = batch?;
        let cols: Vec<ArrayRef> = batch.columns().iter().map(convert_column).collect();
        w.write(&RecordBatch::try_new(out_schema.clone(), cols)?)?;
    }
    w.close()?;
    Ok(())
}

/// Whether a Parquet file has an array column.
fn has_arrays(f: &Path) -> Result<bool> {
    let b = ParquetRecordBatchReaderBuilder::try_new(File::open(f)?)?;
    Ok(b.schema().fields().iter().any(|f| matches!(f.data_type(), DataType::List(_))))
}

/// `arrays_only`: write only the tables that have array columns (the others are the same as
/// in the input).
pub fn convert(input: &Path, output: &Path, arrays_only: bool) -> Result<()> {
    let t0 = Instant::now();
    let mut files = Vec::new();
    parquet_files(input, &mut files)?;
    files.sort();
    if arrays_only {
        let keep: Vec<bool> = files.par_iter().map(|f| has_arrays(f).with_context(|| format!("reading {}", f.display()))).collect::<Result<_>>()?;
        files = files.into_iter().zip(keep).filter(|(_, k)| *k).map(|(f, _)| f).collect();
    }
    files.par_iter().try_for_each(|f| -> Result<()> {
        let dst = output.join(f.strip_prefix(input).unwrap());
        fs::create_dir_all(dst.parent().unwrap())?;
        file(f, &dst).with_context(|| format!("converting {}", f.display()))
    })?;
    eprintln!("wrote {} Parquet files with PostgreSQL array literals to {} in {:.1}s", files.len(), output.display(), t0.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{ListBuilder, StringBuilder};

    #[test]
    fn literals() {
        let mut b = ListBuilder::new(StringBuilder::new());
        b.values().append_value("order.created");
        b.values().append_value("say \"hi\"");
        b.values().append_value("a\\b");
        b.values().append_null();
        b.append(true);
        b.append(true); // empty list
        b.append_null();
        let l = b.finish();
        let c: ArrayRef = Arc::new(l);
        let s = convert_column(&c);
        let s = s.as_string::<i32>();
        assert_eq!(s.value(0), r#"{"order.created","say \"hi\"","a\\b",NULL}"#);
        assert_eq!(s.value(1), "{}");
        assert!(s.is_null(2));
    }
}

fn parquet_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for e in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let p = e?.path();
        if p.is_dir() {
            parquet_files(&p, out)?;
        } else if p.extension().is_some_and(|x| x == "parquet") {
            out.push(p);
        }
    }
    Ok(())
}
