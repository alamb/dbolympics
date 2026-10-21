//! Firebolt Core's query program: its HTTP interface (in its own container), without its result
//! caches. It serves the runner's protocol through ../dbo. A result is
//! Firebolt's CSV. Firebolt ends a statement at its timeout itself (statement_timeout), a second
//! after the deadline, so that the deadline comes first.

// --- The system ----------------------------------------------------------------------------------

const FIREBOLT: &str = "http://127.0.0.1:3473/";
const PARAMS: [(&str, &str); 5] = [("database", "bench"), ("enable_result_cache", "false"), ("enable_subresult_cache", "false"), ("enable_scan_cache", "false"), ("output_format", "CSV")];

struct System(ureq::Agent);
/// nothing to do: Firebolt ends the statement itself
struct Cancel;

impl System {
    fn connect() -> (System, Cancel) {
        // an agent that hands back an error status as a response
        (System(ureq::Agent::config_builder().http_status_as_error(false).build().into()), Cancel)
    }

    fn run(&mut self, sql: &str, timeout: u32) -> Result<Vec<u8>, String> {
        let post = || {
            let r = self.0.post(FIREBOLT).query_pairs(PARAMS);
            let r = if timeout > 0 { r.query("statement_timeout", (timeout + 1000).to_string()) } else { r };
            r.send(sql)
        };
        // a kept connection that the server closed while idle fails at once: then once more
        let mut r = post().or_else(|_| post()).map_err(|e| format!("the connection to Firebolt failed: {e}"))?;
        let body = r.body_mut().with_config().limit(u64::MAX).read_to_vec().map_err(|e| format!("the connection to Firebolt failed: {e}"))?;
        // an error comes as JSON with an "errors" list, whatever the format asked for
        let head = String::from_utf8_lossy(&body[..body.len().min(200)]).trim_start().to_string();
        if r.status() != 200 || head.starts_with("{\"errors\"") || (head.starts_with('{') && String::from_utf8_lossy(&body).contains("\"errors\"")) {
            // on one line: the JSON comes indented
            return Err(String::from_utf8_lossy(&body).split_whitespace().collect::<Vec<_>>().join(" "));
        }
        Ok(body)
    }
}

impl Cancel {
    fn cancel(&self) {}
}

// --- The runner's protocol (../dbo) -------------------------------------------------------------

fn main() {
    let (mut system, cancel) = System::connect();
    dbo::serve(|sql, timeout| system.run(sql, timeout), || cancel.cancel());
}
