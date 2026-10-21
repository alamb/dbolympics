//! The schema for a system, rendered from `schema/schema.json` with the system's dialect: every
//! system gets the same tables and columns, with or without primary keys (the benchmark's
//! `keys`); only type names may differ (the `[dialect.types]` customization points), and a
//! system may append text to every CREATE TABLE (`[ddl]`, e.g. a storage engine). Port of the
//! basic features of `tools/emit_ddl.py` (types, NOT NULL, PRIMARY KEY).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// A type customization point: signature, parameter names, the standard spelling.
struct TypePoint {
    name: &'static str,
    params: &'static [&'static str],
    default: &'static str,
}

const TYPES: &[TypePoint] = &[
    TypePoint { name: "bigint", params: &[], default: "bigint" },
    TypePoint { name: "integer", params: &[], default: "integer" },
    TypePoint { name: "smallint", params: &[], default: "smallint" },
    TypePoint { name: "boolean", params: &[], default: "boolean" },
    TypePoint { name: "date", params: &[], default: "date" },
    TypePoint { name: "timestamp", params: &[], default: "timestamp" },
    TypePoint { name: "uuid", params: &[], default: "uuid" },
    TypePoint { name: "json", params: &[], default: "json" },
    TypePoint { name: "double", params: &[], default: "double precision" },
    TypePoint { name: "varchar", params: &["length"], default: "varchar({length})" },
    TypePoint { name: "decimal", params: &["precision", "scale"], default: "decimal({precision},{scale})" },
    TypePoint { name: "array", params: &["element"], default: "{element} ARRAY" },
];

fn signature(p: &TypePoint) -> String {
    if p.params.is_empty() { p.name.to_string() } else { format!("{}({})", p.name, p.params.join(", ")) }
}

/// Type templates per type name, from the `[dialect.types]` overrides.
pub struct Types(BTreeMap<&'static str, String>);

impl Types {
    pub fn new(overrides: &BTreeMap<String, String>) -> Result<Types> {
        let mut m: BTreeMap<&'static str, String> = TYPES.iter().map(|p| (p.name, p.default.to_string())).collect();
        for (sig, t) in overrides {
            let norm: String = sig.chars().filter(|c| !c.is_whitespace()).collect();
            let p = TYPES.iter().find(|p| signature(p).replace(' ', "") == norm).with_context(|| {
                format!("unknown type customization point {sig:?} (known: {})", TYPES.iter().map(signature).collect::<Vec<_>>().join(", "))
            })?;
            for part in t.split('{').skip(1) {
                let name = part.split('}').next().unwrap_or("");
                if !p.params.contains(&name) {
                    bail!("type template for {sig}: unknown parameter {{{name}}}");
                }
            }
            // a type keeps the schema's precision and scale: only its name may differ
            if p.name == "decimal" && !(t.contains("{precision}") && t.contains("{scale}")) {
                bail!("type template for {sig}: must keep the schema's {{precision}} and {{scale}}");
            }
            m.insert(p.name, t.clone());
        }
        Ok(Types(m))
    }

    fn render(&self, t: &Value) -> Result<String> {
        let name = t["name"].as_str().context("type without name")?;
        let tmpl = self.0.get(name).with_context(|| format!("unknown column type {name}"))?;
        let mut out = tmpl.clone();
        match name {
            "varchar" => out = out.replace("{length}", &t["length"].to_string()),
            "decimal" => out = out.replace("{precision}", &t["precision"].to_string()).replace("{scale}", &t["scale"].to_string()),
            "array" => out = out.replace("{element}", &self.render(&t["element"])?),
            _ => {}
        }
        Ok(out)
    }
}

/// The tables and their CREATE TABLE statements, in module order (lookup tables first within
/// a module). `suffix` follows the closing parenthesis of every statement.
pub fn render(repo: &Path, types: &Types, primary_keys: bool, suffix: &str) -> Result<Vec<(String, String)>> {
    let path = repo.join("schema/schema.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?)?;
    let modules: Vec<&str> = doc["modules"].as_array().context("modules")?.iter().filter_map(|m| m["name"].as_str()).collect();
    let mut tables: Vec<&Value> = doc["tables"].as_array().context("tables")?.iter().collect();
    let rank = |t: &Value| (modules.iter().position(|m| Some(*m) == t["module"].as_str()).unwrap_or(usize::MAX), t["kind"].as_str() != Some("lookup"));
    tables.sort_by_key(|t| rank(t));
    let mut out = Vec::new();
    for t in tables {
        let cols = t["columns"].as_array().context("columns")?;
        // column names always quoted: some (key) are keywords to some systems
        let quote = |n: &str| format!("\"{n}\"");
        let w = cols.iter().filter_map(|c| c["name"].as_str()).map(|n| n.len() + 2).max().unwrap_or(0);
        let mut items = Vec::new();
        for c in cols {
            let mut s = format!("{:<w$} {}", quote(c["name"].as_str().unwrap_or("")), types.render(&c["type"])?);
            if c["nullable"] == Value::Bool(false) {
                s += " NOT NULL";
            }
            items.push(s);
        }
        let pk: Vec<&str> = t["primary_key"].as_array().context("primary_key")?.iter().filter_map(|c| c.as_str()).collect();
        if primary_keys {
            items.push(format!("PRIMARY KEY ({})", pk.iter().map(|c| quote(c)).collect::<Vec<_>>().join(", ")));
        }
        let name = t["name"].as_str().unwrap_or("");
        let tail = if suffix.is_empty() { String::new() } else { format!(" {suffix}") };
        out.push((name.to_string(), format!("CREATE TABLE {name} (\n    {}\n){tail}", items.join(",\n    "))));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rendered schema is the same as the checked-in DDL of tools/emit_ddl.py.
    #[test]
    fn same_as_emit_ddl() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let strip = |sql: &str| -> Vec<String> {
            let code: String = sql.lines().filter(|l| !l.trim_start().starts_with("--")).collect::<Vec<_>>().join("\n");
            code.split(';').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect()
        };
        let sql = |types: &Types, pk: bool, suffix: &str| -> Vec<String> { render(&repo, types, pk, suffix).unwrap().into_iter().map(|(_, s)| s).collect() };
        let none = Types::new(&BTreeMap::new()).unwrap();
        let standard = sql(&none, true, "");
        assert!(sql(&none, false, "ENGINE = x").iter().all(|s| !s.contains("PRIMARY KEY") && s.ends_with(") ENGINE = x")));
        assert_eq!(standard, strip(&std::fs::read_to_string(repo.join("schema/ddl/standard.sql")).unwrap()));
        let pg = Types::new(&[("json".to_string(), "jsonb".to_string())].into()).unwrap();
        assert_eq!(sql(&pg, true, ""), strip(&std::fs::read_to_string(repo.join("schema/ddl/postgres.sql")).unwrap()));
    }

    #[test]
    fn overrides() {
        let t = Types::new(&[("varchar(length)".to_string(), "nvarchar({length})".to_string()), ("array(element)".to_string(), "{element}[]".to_string())].into()).unwrap();
        let v: Value = serde_json::from_str(r#"{"name": "array", "element": {"name": "varchar", "length": 64}}"#).unwrap();
        assert_eq!(t.render(&v).unwrap(), "nvarchar(64)[]");
        assert!(Types::new(&[("varchar".to_string(), "text".to_string())].into()).is_err());
        assert!(Types::new(&[("varchar(length)".to_string(), "varchar({len})".to_string())].into()).is_err());
        assert!(Types::new(&[("decimal(precision, scale)".to_string(), "Decimal(38, 6)".to_string())].into()).is_err());
        assert!(Types::new(&[("decimal(precision, scale)".to_string(), "Decimal({precision}, {scale})".to_string())].into()).is_ok());
    }
}
