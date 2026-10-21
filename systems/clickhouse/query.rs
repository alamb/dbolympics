//! ClickHouse's query program: ClickHouse's HTTP interface, one session for the run. It serves the
//! runner's protocol (../dbo). A result is ClickHouse's CSV; a
//! timeout kills the statement.

// --- The system ----------------------------------------------------------------------------------

const CLICKHOUSE: &str = "http://127.0.0.1:8123/";

struct System(ureq::Agent);
struct Cancel(ureq::Agent);

impl System {
    fn connect() -> (System, Cancel) {
        let ch = clickhouse_agent();
        dbo::retry(|| clickhouse_run(&ch, "SELECT 1"));
        (System(ch), Cancel(clickhouse_agent()))
    }

    fn run(&mut self, sql: &str, _timeout: u32) -> Result<Vec<u8>, String> {
        clickhouse_run(&self.0, sql)
    }
}

impl Cancel {
    fn cancel(&self) {
        clickhouse_cancel(&self.0);
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
