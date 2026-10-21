//! Runtime view of schema/schema.json (embedded at compile time).

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    BigInt,
    Integer,
    SmallInt,
    Boolean,
    Decimal { precision: u8, scale: i8 },
    Double,
    Varchar(usize),
    Date,
    Timestamp,
    Uuid,
    Json,
    Array(Box<Type>),
}

#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    pub ty: Type,
    pub nullable: bool,
}

#[derive(Clone, Debug)]
pub struct ForeignKey {
    pub column: usize,
    pub table: TableId,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub id: TableId,
    pub name: String,
    pub module: String,
    #[allow(dead_code)] // part of the data model, not used by the simulator yet
    pub is_lookup: bool,
    pub columns: Vec<Column>,
    /// True if the primary key is a single bigint column named `id` at index 0 (assigned by
    /// the engine).
    pub has_id: bool,
    pub foreign_keys: Vec<ForeignKey>,
    pub lookup_rows: Vec<Vec<serde_json::Value>>,
}

pub struct Schema {
    pub tables: Vec<Table>,
    /// tables referenced by at least one single-column FK to `id`
    pub is_fk_target: Vec<bool>,
}

#[derive(Deserialize)]
struct JDoc {
    tables: Vec<JTable>,
}
#[derive(Deserialize)]
struct JTable {
    name: String,
    module: String,
    kind: String,
    columns: Vec<JColumn>,
    primary_key: Vec<String>,
    #[serde(default)]
    foreign_keys: Vec<JFk>,
    #[serde(default)]
    rows: Vec<Vec<serde_json::Value>>,
}
#[derive(Deserialize)]
struct JColumn {
    name: String,
    #[serde(rename = "type")]
    ty: JType,
    nullable: bool,
}
#[derive(Deserialize)]
struct JType {
    name: String,
    length: Option<usize>,
    precision: Option<u8>,
    scale: Option<i8>,
    element: Option<Box<JType>>,
}
#[derive(Deserialize)]
struct JFk {
    columns: Vec<String>,
    references: JRef,
}
#[derive(Deserialize)]
struct JRef {
    table: String,
    columns: Vec<String>,
}

fn to_type(t: &JType) -> Type {
    match t.name.as_str() {
        "bigint" => Type::BigInt,
        "integer" => Type::Integer,
        "smallint" => Type::SmallInt,
        "boolean" => Type::Boolean,
        "decimal" => Type::Decimal { precision: t.precision.unwrap(), scale: t.scale.unwrap() },
        "double" => Type::Double,
        "varchar" => Type::Varchar(t.length.unwrap()),
        "date" => Type::Date,
        "timestamp" => Type::Timestamp,
        "uuid" => Type::Uuid,
        "json" => Type::Json,
        "array" => Type::Array(Box::new(to_type(t.element.as_ref().unwrap()))),
        other => panic!("unknown type {other}"),
    }
}

const SCHEMA_JSON: &str = include_str!("../schema/schema.json");

pub fn schema() -> &'static Schema {
    static S: OnceLock<Schema> = OnceLock::new();
    S.get_or_init(|| {
        let doc: JDoc = serde_json::from_str(SCHEMA_JSON).expect("schema.json");
        let index: std::collections::HashMap<&str, usize> =
            doc.tables.iter().enumerate().map(|(i, t)| (t.name.as_str(), i)).collect();
        let mut tables = Vec::new();
        let mut is_fk_target = vec![false; doc.tables.len()];
        for (i, t) in doc.tables.iter().enumerate() {
            let columns: Vec<Column> = t
                .columns
                .iter()
                .map(|c| Column { name: c.name.clone(), ty: to_type(&c.ty), nullable: c.nullable })
                .collect();
            let col = |n: &str| columns.iter().position(|c| c.name == n).unwrap();
            let has_id = t.primary_key == ["id"] && columns[0].name == "id" && columns[0].ty == Type::BigInt;
            let mut foreign_keys = Vec::new();
            for fk in &t.foreign_keys {
                if fk.columns.len() == 1 && fk.references.columns == ["id"] {
                    let target = index[fk.references.table.as_str()];
                    foreign_keys.push(ForeignKey { column: col(&fk.columns[0]), table: TableId(target) });
                    is_fk_target[target] = true;
                }
            }
            tables.push(Table {
                id: TableId(i),
                name: t.name.clone(),
                module: t.module.clone(),
                is_lookup: t.kind == "lookup",
                columns,
                has_id,
                foreign_keys,
                lookup_rows: t.rows.clone(),
            });
        }
        Schema { tables, is_fk_target }
    })
}

pub fn table(t: TableId) -> &'static Table {
    &schema().tables[t.0]
}
