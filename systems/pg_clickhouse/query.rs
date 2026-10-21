//! pg_clickhouse's query program: ClickHouse holds the data, PostgreSQL runs the queries on it
//! through the extension. It serves the runner's protocol (../dbo)
//!. DDL and loads go to ClickHouse (its HTTP interface); after a CREATE TABLE,
//! PostgreSQL imports the table (IMPORT FOREIGN SCHEMA). Everything else goes to PostgreSQL (the
//! simple query protocol, values as text), where a timeout cancels.

use dbo::pg::{Pg, PgCancel};

// --- The system ----------------------------------------------------------------------------------

/// ClickHouse's HTTP interface; PostgreSQL's socket (its container's, in /run/dbo)
const CLICKHOUSE: &str = "http://127.0.0.1:8123/";
const POSTGRES: &str = "host=/run/dbo port=5432 user=postgres password=Dbolympics-Bench-2026! dbname=postgres";

struct System {
    ch: ureq::Agent,
    pg: Pg,
}
struct Cancel(PgCancel);

impl System {
    fn connect() -> (System, Cancel) {
        let ch = clickhouse_agent();
        dbo::retry(|| clickhouse_run(&ch, "SELECT 1"));
        let pg = Pg::connect(POSTGRES);
        let cancel = Cancel(pg.canceller());
        (System { ch, pg }, cancel)
    }

    fn run(&mut self, sql: &str, _timeout: u32) -> Result<Vec<u8>, String> {
        // DDL and loads (the load statement of system.toml) to ClickHouse
        if let Some(rest) = sql.strip_prefix("CREATE TABLE ") {
            clickhouse_run(&self.ch, sql)?;
            let table = rest.split_whitespace().next().unwrap_or("");
            self.pg.query(&format!("IMPORT FOREIGN SCHEMA \"default\" LIMIT TO ({table}) FROM SERVER ch_srv INTO public"))?;
            return Ok(Vec::new());
        }
        if sql.starts_with("INSERT INTO ") {
            return clickhouse_run(&self.ch, sql);
        }
        self.pg.run(sql)
    }
}

impl Cancel {
    /// Only queries have a timeout, and they run in PostgreSQL: the key on a new connection, as
    /// libpq (dbo::pg).
    fn cancel(&self) {
        self.0.cancel();
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

