//! clickhouse_brew's query program (Querybrew on ClickHouse): Umbra writes every query for
//! ClickHouse, and ClickHouse runs what Umbra wrote. It serves the runner's protocol
//! (../dbo). Both hold the data:
//! - a statement runs in Umbra; DDL and loads also in ClickHouse (types and table engine as for
//!   plain ClickHouse; a Parquet file with file(), the original with arrays where Umbra got them as
//!   text);
//! - a query goes to Umbra as EXPLAIN (FORMAT SQL, DIALECT clickhouse), and its text runs in
//!   ClickHouse.
//!
//! Umbra over the PostgreSQL protocol (values as text), ClickHouse over its HTTP interface (its
//! CSV). A timeout cancels in Umbra and kills the statement in ClickHouse.

use dbo::pg::{Pg, PgCancel};
use regex::Regex;

// --- The system ----------------------------------------------------------------------------------

/// Umbra's socket (its container's, in /run/dbo); ClickHouse's HTTP interface
const UMBRA: &str = "host=/run/dbo port=5432 user=postgres password=postgres dbname=postgres";
const CLICKHOUSE: &str = "http://127.0.0.1:8123/";

struct System {
    umbra: Pg,
    ch: ureq::Agent,
    copy: Regex,
    /// Umbra's column types, as ClickHouse's
    types: Vec<(Regex, &'static str)>,
}
struct Cancel(PgCancel, ureq::Agent);

impl System {
    fn connect() -> (System, Cancel) {
        let umbra = Pg::connect(UMBRA);
        let ch = clickhouse_agent();
        dbo::retry(|| clickhouse_run(&ch, "SELECT 1"));
        let cancel = Cancel(umbra.canceller(), clickhouse_agent());
        let copy = Regex::new(r"^COPY (\w+) FROM '([^']+)' WITH \(FORMAT parquet\)$").unwrap();
        let types = [
            (r"varchar\(\d+\) ARRAY\b", "Array(String)"),
            (r"varchar\(\d+\)", "String"),
            (r"json\b", "String"),
            (r"timestamp\b", "DateTime64(6)"),
            (r"date\b", "Date32"),
            (r"uuid\b", "UUID"),
        ];
        let types = types.into_iter().map(|(t, ch)| (Regex::new(&format!(r#"(?m)^(\s+"\w+"\s+){t}"#)).unwrap(), ch)).collect();
        (System { umbra, ch, copy, types }, cancel)
    }

    fn run(&mut self, sql: &str, _timeout: u32) -> Result<Vec<u8>, String> {
        // a query: anything but DDL, loads and settings
        if !["CREATE TABLE ", "COPY ", "SET "].iter().any(|p| sql.starts_with(p)) {
            let explain = self.umbra.query(&format!("EXPLAIN (FORMAT SQL, DIALECT clickhouse) {sql}")).map_err(|e| format!("Umbra could not write the query: {e}"))?;
            let written = dbo::pg::lines(&explain);
            return clickhouse_run(&self.ch, &written).map_err(|e| format!("-- the query as Umbra wrote it:\n{written}\n{e}"));
        }
        self.umbra.query(sql).map_err(|e| format!("in Umbra: {e}"))?;
        let target = if let Some(m) = self.copy.captures(sql) {
            format!("INSERT INTO {} SELECT * FROM file('{}', Parquet)", &m[1], m[2].replace("/data-arrays/", "/data/"))
        } else if sql.starts_with("CREATE TABLE ") {
            let mut t = sql.to_string();
            for (re, ty) in &self.types {
                t = re.replace_all(&t, format!("${{1}}{ty}")).into_owned();
            }
            if t.contains("PRIMARY KEY") { format!("{t} ENGINE = MergeTree") } else { format!("{t} ENGINE = MergeTree ORDER BY tuple()") }
        } else {
            return Ok(Vec::new()); // a setting: Umbra's, for writing the queries
        };
        clickhouse_run(&self.ch, &target).map_err(|e| format!("in ClickHouse: {e}"))?;
        Ok(Vec::new())
    }
}

impl Cancel {
    /// In Umbra the key on a new connection, as libpq (dbo::pg); in ClickHouse a kill.
    fn cancel(&self) {
        self.0.cancel();
        clickhouse_cancel(&self.1);
    }
}

// --- The runner's protocol (../dbo) -------------------------------------------------------------

fn main() {
    let (mut system, cancel) = System::connect();
    dbo::serve(|sql, timeout| system.run(sql, timeout), || cancel.cancel());
}

/// ClickHouse's HTTP interface, one session: a result is ClickHouse's CSV. Every statement has the
/// same ID, so that the one running can be killed by it (KILL QUERY ... SYNC waits for its end,
/// before the next one starts).
fn clickhouse_run(agent: &ureq::Agent, sql: &str) -> Result<Vec<u8>, String> {
    let post = || {
        // the whole result before the status, so that an error is a status and not the end of the body
        let params = [("session_id", "dbo"), ("session_timeout", "3600"), ("wait_end_of_query", "1"), ("default_format", "CSV"), ("query_id", "dbo")];
        agent.post(CLICKHOUSE).query_pairs(params).send(sql)
    };
    // a kept connection that the server closed while idle fails at once: then once more
    let mut r = post().or_else(|_| post()).map_err(|e| format!("the connection to ClickHouse failed: {e}"))?;
    let body = r.body_mut().with_config().limit(u64::MAX).read_to_vec().map_err(|e| format!("the connection to ClickHouse failed: {e}"))?;
    if r.status() != 200 {
        return Err(String::from_utf8_lossy(&body).trim().to_string());
    }
    Ok(body)
}

/// An agent that hands back an error status as a response.
fn clickhouse_agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).build().into()
}

/// Kill the running statement in ClickHouse.
fn clickhouse_cancel(agent: &ureq::Agent) {
    let _ = agent.post(CLICKHOUSE).send("KILL QUERY WHERE query_id = 'dbo' SYNC");
}

