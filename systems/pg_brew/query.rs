//! pg_brew's query program (Querybrew on PostgreSQL): Umbra writes every query for PostgreSQL, and
//! PostgreSQL runs what Umbra wrote. It serves the runner's protocol (../dbo)
//!. Both hold the data:
//! - a statement runs in Umbra; DDL and loads also in PostgreSQL (json columns as jsonb; a Parquet
//!   file through pg_duckdb, as for plain PostgreSQL);
//! - a query goes to Umbra as EXPLAIN (FORMAT SQL, DIALECT postgresql), and its text runs in
//!   PostgreSQL.
//!
//! Both speak the PostgreSQL protocol (the simple query protocol, values as text); a timeout
//! cancels in both.

use dbo::pg::{Pg, PgCancel};
use regex::Regex;

// --- The system ----------------------------------------------------------------------------------

/// their sockets (their containers', in /run/dbo)
const UMBRA: &str = "host=/run/dbo port=5432 user=postgres password=postgres dbname=postgres";
const POSTGRES: &str = "host=/run/dbo port=5433 user=postgres password=Dbolympics-Bench-2026! dbname=postgres";

struct System {
    umbra: Pg,
    pg: Pg,
    copy: Regex,
    json: Regex,
}
struct Cancel(PgCancel, PgCancel);

impl System {
    fn connect() -> (System, Cancel) {
        let umbra = Pg::connect(UMBRA);
        let pg = Pg::connect(POSTGRES);
        let cancel = Cancel(umbra.canceller(), pg.canceller());
        let copy = Regex::new(r"^COPY (\w+) FROM '([^']+)' WITH \(FORMAT parquet\)$").unwrap();
        let json = Regex::new(r#"(?m)^(\s+"\w+"\s+)json\b"#).unwrap();
        (System { umbra, pg, copy, json }, cancel)
    }

    fn run(&mut self, sql: &str, _timeout: u32) -> Result<Vec<u8>, String> {
        // a query: anything but DDL, loads and settings
        if !["CREATE TABLE ", "COPY ", "SET "].iter().any(|p| sql.starts_with(p)) {
            let explain = self.umbra.query(&format!("EXPLAIN (FORMAT SQL, DIALECT postgresql) {sql}")).map_err(|e| format!("Umbra could not write the query: {e}"))?;
            let written = dbo::pg::lines(&explain);
            return self.pg.run(&written).map_err(|e| format!("-- the query as Umbra wrote it:\n{written}\n{e}"));
        }
        self.umbra.query(sql).map_err(|e| format!("in Umbra: {e}"))?;
        let target = if let Some(m) = self.copy.captures(sql) {
            format!("COPY (SELECT * FROM read_parquet('{}')) TO '/tmp/dbo_stage.csv' (FORMAT csv, HEADER false); COPY {} FROM '/tmp/dbo_stage.csv' (FORMAT csv)", &m[2], &m[1])
        } else if sql.starts_with("CREATE TABLE ") {
            self.json.replace_all(sql, "${1}jsonb").into_owned()
        } else {
            return Ok(Vec::new()); // a setting: Umbra's, for writing the queries
        };
        self.pg.query(&target).map_err(|e| format!("in PostgreSQL: {e}"))?;
        Ok(Vec::new())
    }
}

impl Cancel {
    /// In both (dbo::pg: the key on a new connection, as libpq).
    fn cancel(&self) {
        self.0.cancel();
        self.1.cancel();
    }
}

// --- The runner's protocol (../dbo) -------------------------------------------------------------

fn main() {
    let (mut system, cancel) = System::connect();
    dbo::serve(|sql, timeout| system.run(sql, timeout), || cancel.cancel());
}

