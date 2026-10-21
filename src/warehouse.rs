//! The "warehouse" of the query-generation role play (docs/QUERY-GENERATION.md): it takes one
//! report request (title, question, SQL), checks it like a data-engineering reviewer would,
//! runs it on DuckDB and returns a printed report capped at 50 rows. Every request, run or
//! bounced, is appended to a lineage log.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use duckdb::Connection;
use serde_json::Value as J;

/// Printed page limits.
const MAX_ROWS: usize = 50;
const MAX_COLUMNS: usize = 12;

/// Functions the warehouse pretends not to have, because many engines lack them (series
/// generators: use `calendar_days` or a recursive CTE). Rejected like an unknown function.
const UNAVAILABLE_FUNCTIONS: [&str; 2] = ["generate_series", "range"];

/// The empty copy of the schema next to a warehouse file, used by dry runs:
/// `tmp/sf1.duckdb` -> `tmp/sf1.empty.duckdb`.
pub fn empty_path(db: &Path) -> PathBuf {
    db.with_extension("empty.duckdb")
}

/// Build a DuckDB database file from a generated dataset (plain schema, no constraints),
/// plus an empty copy of the schema for dry runs.
pub fn load(dir: &Path, db: &Path) -> Result<()> {
    let t0 = Instant::now();
    let ddl = fs::read_to_string("schema/ddl/duckdb.sql").context("reading duckdb.sql")?;
    let empty = empty_path(db);
    for p in [db, empty.as_path()] {
        if p.exists() {
            fs::remove_file(p)?;
        }
    }
    Connection::open(&empty)?.execute_batch(&ddl).context("creating the empty schema")?;
    let conn = Connection::open(db)?;
    conn.execute_batch(&ddl).context("creating the schema")?;
    let mut total = 0usize;
    for t in &crate::schema::schema().tables {
        let glob = dir.join(&t.name).join("*.parquet");
        let n = conn.execute(&format!("INSERT INTO {} SELECT * FROM read_parquet('{}')", t.name, glob.display()), []).with_context(|| format!("loading {}", t.name))?;
        total += n;
    }
    conn.execute_batch("CHECKPOINT")?;
    println!("loaded {total} rows into {} (empty schema for dry runs: {}) in {:.1}s", db.display(), empty.display(), t0.elapsed().as_secs_f64());
    Ok(())
}

/// What the parser found in the SQL itself.
#[derive(Default, Debug)]
struct Shape {
    tables: BTreeSet<String>,
    /// base tables of each branch of a top-level UNION / INTERSECT / EXCEPT
    branches: Vec<BTreeSet<String>>,
    /// scalar subqueries in the final select list
    scalar_subqueries: usize,
    /// whether the final select has a FROM clause
    has_from: bool,
}

fn base_tables(n: &J, out: &mut BTreeSet<String>) {
    match n {
        J::Object(m) => {
            if m.get("type").and_then(J::as_str) == Some("BASE_TABLE") {
                if let Some(t) = m.get("table_name").and_then(J::as_str) {
                    out.insert(t.to_lowercase());
                }
            }
            for v in m.values() {
                base_tables(v, out);
            }
        }
        J::Array(a) => a.iter().for_each(|v| base_tables(v, out)),
        _ => {}
    }
}

/// Names of all CTEs defined anywhere in the query (they parse as base tables when used).
fn cte_names(n: &J, out: &mut BTreeSet<String>) {
    match n {
        J::Object(m) => {
            if let Some(map) = m.get("cte_map").and_then(|c| c.get("map")).and_then(J::as_array) {
                out.extend(map.iter().filter_map(|e| e.get("key").and_then(J::as_str)).map(str::to_lowercase));
            }
            m.values().for_each(|v| cte_names(v, out));
        }
        J::Array(a) => a.iter().for_each(|v| cte_names(v, out)),
        _ => {}
    }
}

/// Names of all functions called anywhere in the query (scalar and table functions).
fn function_names(n: &J, out: &mut BTreeSet<String>) {
    match n {
        J::Object(m) => {
            if let Some(f) = m.get("function_name").and_then(J::as_str) {
                out.insert(f.to_lowercase());
            }
            m.values().for_each(|v| function_names(v, out));
        }
        J::Array(a) => a.iter().for_each(|v| function_names(v, out)),
        _ => {}
    }
}

/// Branches of a (possibly nested) set operation.
fn set_branches(n: &J, out: &mut Vec<J>) {
    if n.get("type").and_then(J::as_str) == Some("SET_OPERATION_NODE") {
        for side in ["left", "right"] {
            if let Some(c) = n.get(side) {
                set_branches(c, out);
            }
        }
    } else {
        out.push(n.clone());
    }
}

fn analyze(conn: &Connection, sql: &str) -> Result<Shape> {
    let parsed: String = conn.query_row("SELECT json_serialize_sql(CAST(? AS VARCHAR))", [sql], |r| r.get(0))?;
    let j: J = serde_json::from_str(&parsed)?;
    if j.get("error").and_then(J::as_bool) == Some(true) {
        bail!("not a single SELECT statement: {}", j.get("error_message").and_then(J::as_str).unwrap_or("parse error"));
    }
    let stmts = j.get("statements").and_then(J::as_array).map(|a| a.as_slice()).unwrap_or(&[]);
    if stmts.len() != 1 {
        bail!("send exactly one SELECT statement per request");
    }
    let node = &stmts[0]["node"];
    let mut funcs = BTreeSet::new();
    function_names(node, &mut funcs);
    if let Some(f) = UNAVAILABLE_FUNCTIONS.iter().find(|f| funcs.contains(**f)) {
        bail!("does not compile: Catalog Error: Function with name {f} does not exist!");
    }
    let mut shape = Shape::default();
    let mut ctes = BTreeSet::new();
    cte_names(node, &mut ctes);
    base_tables(node, &mut shape.tables);
    shape.tables.retain(|t| !ctes.contains(t));
    let mut branches = Vec::new();
    set_branches(node, &mut branches);
    if branches.len() > 1 {
        for b in &branches {
            let mut t = BTreeSet::new();
            base_tables(b, &mut t);
            t.retain(|t| !ctes.contains(t));
            shape.branches.push(t);
        }
    }
    if let Some(list) = node.get("select_list").and_then(J::as_array) {
        shape.scalar_subqueries = list.iter().filter(|e| e.get("class").and_then(J::as_str) == Some("SUBQUERY") && e.get("subquery_type").and_then(J::as_str) == Some("SCALAR")).count();
    }
    shape.has_from = node.get("from_table").and_then(|f| f.get("type")).and_then(J::as_str).is_some_and(|t| t != "EMPTY");
    Ok(shape)
}

/// Signs of bundled questions that are not bounced (docs/QUERY-GENERATION.md, "The warehouse"): logged and
/// printed on stderr for us, never shown to the requester.
fn warnings(shape: &Shape) -> Vec<String> {
    let mut w = Vec::new();
    'outer: for (i, a) in shape.branches.iter().enumerate() {
        for b in &shape.branches[i + 1..] {
            if a.is_disjoint(b) {
                let list = |s: &BTreeSet<String>| s.iter().cloned().collect::<Vec<_>>().join(", ");
                w.push(format!("union of unrelated queries ({} vs {})", list(a), list(b)));
                break 'outer;
            }
        }
    }
    if shape.scalar_subqueries >= 3 || (shape.scalar_subqueries >= 2 && !shape.has_from) {
        let from = if shape.has_from { "over a FROM, may be per-row values" } else { "no FROM" };
        w.push(format!("{} scalar subqueries in the select list ({from})", shape.scalar_subqueries));
    }
    w
}

pub struct Request<'a> {
    pub title: &'a str,
    pub question: &'a str,
    pub sql: &'a str,
    pub dry_run: bool,
    pub log: Option<&'a Path>,
    pub tags: Option<&'a str>,
}

/// Review, run and print one report request. Returns the printed report.
pub fn query(db: &Path, req: &Request) -> Result<String> {
    let t0 = Instant::now();
    // dry runs execute against the empty copy of the schema: they catch errors that only
    // show when a query runs (not just binds), without touching or showing any data
    let path = if req.dry_run { empty_path(db) } else { db.to_path_buf() };
    if !path.exists() {
        bail!("{} not found: run `dbolympics load` again", path.display());
    }
    // read-only, so several requests can run against the same file at once
    let conn = Connection::open_with_flags(&path, duckdb::Config::default().access_mode(duckdb::AccessMode::ReadOnly)?)?;
    // standard JSON_VALUE returns the scalar as text; DuckDB's returns it JSON-quoted, which
    // makes `json_value(p, '$.x') = 'abc'` silently match nothing
    conn.execute_batch("CREATE TEMP MACRO json_value(j, p) AS json_extract_string(j, p)")?;
    let mut out = String::new();
    out.push_str(&format!("REPORT: {}\nQUESTION: {}\n", req.title, req.question));
    let mut verdict = "ok";
    let mut reasons: Vec<String> = Vec::new();
    let mut rows_total = 0usize;
    let mut columns: Vec<String> = Vec::new();
    let mut shape = Shape::default();
    let mut warns: Vec<String> = Vec::new();
    match analyze(&conn, req.sql) {
        Err(e) => {
            verdict = "rejected";
            reasons.push(e.to_string());
        }
        Ok(s) => {
            warns = warnings(&s);
            shape = s;
        }
    }
    // dry run: does it bind?
    if verdict == "ok" {
        if let Err(e) = conn.execute_batch(&format!("EXPLAIN {}", req.sql)) {
            verdict = "rejected";
            reasons.push(format!("does not compile: {}", first_line(&e.to_string())));
        }
    }
    // the dry run runs the query on the empty schema and shows the output columns (and
    // whether they fit the page)
    if verdict == "ok" && req.dry_run {
        match conn.execute_batch(&format!("CREATE TEMP TABLE __dry AS {}", req.sql)) {
            Err(e) => {
                verdict = "rejected";
                reasons.push(format!("does not run: {}", first_line(&e.to_string())));
            }
            Ok(()) => {
                let mut st = conn.prepare("SELECT column_name FROM (DESCRIBE __dry)")?;
                columns = st.query_map([], |r| r.get::<_, String>(0))?.collect::<std::result::Result<_, _>>()?;
                if columns.len() > MAX_COLUMNS {
                    verdict = "bounced";
                    reasons.push(format!("{} columns do not fit the page (at most {MAX_COLUMNS})", columns.len()));
                }
            }
        }
    }
    if verdict == "ok" && !req.dry_run {
        match conn.execute_batch(&format!("CREATE TEMP TABLE __report AS {}", req.sql)) {
            Err(e) => {
                verdict = "failed";
                reasons.push(format!("execution failed: {}", first_line(&e.to_string())));
            }
            Ok(()) => {
                let mut st = conn.prepare("SELECT column_name FROM (DESCRIBE __report)")?;
                columns = st.query_map([], |r| r.get::<_, String>(0))?.collect::<std::result::Result<_, _>>()?;
                rows_total = conn.query_row("SELECT count(*) FROM __report", [], |r| r.get::<_, i64>(0))? as usize;
                if columns.len() > MAX_COLUMNS {
                    verdict = "bounced";
                    reasons.push(format!("{} columns do not fit the page (at most {MAX_COLUMNS})", columns.len()));
                }
                // mostly-empty columns can be bundled questions, but also an ordinary detail
                // report (error codes of successful attempts...): a hidden warning, no bounce
                if rows_total >= 4 {
                    let mut empty = Vec::new();
                    for c in &columns {
                        let nulls: i64 = conn.query_row(&format!("SELECT count(*) FROM __report WHERE \"{}\" IS NULL", c.replace('"', "\"\"")), [], |r| r.get(0))?;
                        if nulls as f64 >= 0.8 * rows_total as f64 {
                            empty.push(c.clone());
                        }
                    }
                    if empty.len() >= 2 {
                        warns.push(format!("columns {} are mostly empty", empty.join(", ")));
                    }
                }
                if verdict == "ok" {
                    out.push_str(&render(&conn, &columns, rows_total)?);
                }
            }
        }
    }
    if verdict != "ok" {
        out.push_str(&format!("STATUS: {}\n", verdict.to_uppercase()));
        for r in &reasons {
            out.push_str(&format!("  - {r}\n"));
        }
    } else if req.dry_run {
        out.push_str(&format!("STATUS: COMPILES (dry run on the empty schema, no data read)\nCOLUMNS: {}\n", columns.join(", ")));
    }
    if let Some(log) = req.log {
        let entry = serde_json::json!({
            "title": req.title,
            "question": req.question,
            "sql": req.sql,
            "tags": req.tags.map(|t| serde_json::from_str::<J>(t).unwrap_or(J::String(t.to_string()))),
            "dry_run": req.dry_run,
            "verdict": verdict,
            "reasons": reasons,
            "warnings": warns,
            "tables": shape.tables,
            "columns": columns,
            "rows": rows_total,
            "elapsed_ms": t0.elapsed().as_millis() as u64,
        });
        // one write per record: concurrent requests append whole lines (O_APPEND)
        let mut f = fs::OpenOptions::new().create(true).append(true).open(log)?;
        f.write_all(format!("{entry}\n").as_bytes())?;
    }
    for w in &warns {
        eprintln!("WARNING: {w}");
    }
    Ok(out)
}

fn first_line(s: &str) -> String {
    s.lines().find(|l| !l.trim().is_empty()).unwrap_or(s).chars().take(300).collect()
}

/// The printed table: right-aligned numbers, at most MAX_ROWS rows.
fn render(conn: &Connection, columns: &[String], total: usize) -> Result<String> {
    let casts = columns.iter().map(|c| format!("coalesce(CAST(\"{0}\" AS VARCHAR), 'NULL')", c.replace('"', "\"\""))).collect::<Vec<_>>().join(", ");
    let mut st = conn.prepare(&format!("SELECT {casts} FROM __report LIMIT {MAX_ROWS}"))?;
    let n = columns.len();
    let rows: Vec<Vec<String>> = st.query_map([], |r| (0..n).map(|i| r.get::<_, String>(i)).collect())?.collect::<std::result::Result<_, _>>()?;
    let numeric: Vec<bool> = (0..n).map(|i| rows.iter().all(|r| r[i] == "NULL" || r[i].parse::<f64>().is_ok())).collect();
    let width: Vec<usize> = (0..n).map(|i| rows.iter().map(|r| r[i].chars().count()).chain([columns[i].chars().count()]).max().unwrap_or(1).min(40)).collect();
    let cell = |s: &str, i: usize| {
        let s: String = s.chars().take(40).collect();
        if numeric[i] { format!("{:>w$}", s, w = width[i]) } else { format!("{:<w$}", s, w = width[i]) }
    };
    let mut out = String::new();
    out.push_str(&(0..n).map(|i| cell(&columns[i], i)).collect::<Vec<_>>().join(" | "));
    out.push('\n');
    out.push_str(&width.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("-+-"));
    out.push('\n');
    for r in &rows {
        out.push_str(&(0..n).map(|i| cell(&r[i], i)).collect::<Vec<_>>().join(" | "));
        out.push('\n');
    }
    out.push_str(&if total > MAX_ROWS { format!("({total} rows, first {MAX_ROWS} printed)\n") } else { format!("({total} rows)\n") });
    Ok(out)
}

/// Profile queries for the catalog (tools/catalog.py): for each input line `{"key", "sql"}`,
/// write `{"key", "plan", "rows", "columns", "ms", "fingerprint", "error"}`. The plan is
/// DuckDB's optimised physical plan as JSON. The fingerprint is order-independent: each
/// column's values are normalised (numbers to 6 significant digits), sorted and hashed, and
/// the column hashes are hashed in sorted order, so rewrites with other column or row order
/// or names compare equal.
pub fn profile(db: &Path, input: &Path, output: &Path) -> Result<()> {
    let conn = Connection::open_with_flags(db, duckdb::Config::default().access_mode(duckdb::AccessMode::ReadOnly)?)?;
    conn.execute_batch("CREATE TEMP MACRO json_value(j, p) AS json_extract_string(j, p)")?;
    let mut out = fs::File::create(output)?;
    let text = fs::read_to_string(input)?;
    let n = text.lines().count();
    for (i, line) in text.lines().enumerate() {
        let q: J = serde_json::from_str(line)?;
        let key = q["key"].as_str().unwrap_or("").to_string();
        let sql = q["sql"].as_str().unwrap_or("").trim().trim_end_matches(';').to_string();
        let mut rec = serde_json::json!({"key": key});
        let t0 = Instant::now();
        let run = (|| -> Result<()> {
            let plan: String = conn.query_row(&format!("EXPLAIN (FORMAT JSON) {sql}"), [], |r| r.get(1))?;
            rec["plan"] = serde_json::from_str(&plan).unwrap_or(J::String(plan));
            conn.execute_batch("DROP TABLE IF EXISTS __p")?;
            let t1 = Instant::now();
            conn.execute_batch(&format!("CREATE TEMP TABLE __p AS {sql}"))?;
            rec["ms"] = J::from(t1.elapsed().as_millis() as u64);
            let mut st = conn.prepare("SELECT column_name, column_type FROM (DESCRIBE __p)")?;
            let cols: Vec<(String, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?;
            rec["columns"] = J::from(cols.len());
            rec["rows"] = J::from(conn.query_row("SELECT count(*) FROM __p", [], |r| r.get::<_, i64>(0))?);
            let numeric = |t: &str| ["INT", "DECIMAL", "DOUBLE", "FLOAT", "REAL", "NUMERIC", "HUGEINT", "BIGINT"].iter().any(|k| t.to_uppercase().contains(k));
            let mut hashes = Vec::new();
            let mut per_column = Vec::new();
            for (c, t) in &cols {
                let q = format!("\"{}\"", c.replace('"', "\"\""));
                let v = if numeric(t) { format!("printf('%.6g', CAST({q} AS DOUBLE))") } else { format!("CAST({q} AS VARCHAR)") };
                let (h, distinct): (String, i64) = conn.query_row(&format!("SELECT md5(coalesce(string_agg(coalesce({v}, '<null>'), '|' ORDER BY coalesce({v}, '<null>')), '')), count(DISTINCT {q}) FROM __p"), [], |r| Ok((r.get(0)?, r.get(1)?)))?;
                per_column.push(serde_json::json!({"name": c, "numeric": numeric(t), "distinct": distinct, "hash": h}));
                hashes.push(h);
            }
            rec["column_fingerprints"] = J::Array(per_column);
            hashes.sort();
            rec["fingerprint"] = J::String(conn.query_row("SELECT md5(?)", [hashes.join(",")], |r| r.get::<_, String>(0))?);
            Ok(())
        })();
        if let Err(e) = run {
            rec["error"] = J::String(first_line(&e.to_string()));
        }
        rec["total_ms"] = J::from(t0.elapsed().as_millis() as u64);
        writeln!(out, "{rec}")?;
        if (i + 1) % 100 == 0 {
            eprintln!("profiled {}/{n}", i + 1);
        }
    }
    Ok(())
}
