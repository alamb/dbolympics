//! chDB's query program: chDB (ClickHouse as a library) embedded, one session for the whole run. It
//! serves the runner's protocol (../dbo). A result is ClickHouse's
//! CSV. chDB cannot cancel a statement from another thread: it ends one at its timeout itself
//! (max_execution_time), a second after the deadline, so that the deadline comes first.

use chdb_rust::arg::Arg;
use chdb_rust::format::OutputFormat;
use chdb_rust::session::{Session, SessionBuilder};

// --- The system ----------------------------------------------------------------------------------

const DATA: &str = "/system/db";

/// the session, and the timeout its time limit is set for
struct System(Session, Option<u32>);
/// nothing to do: chDB ends the statement itself
struct Cancel;

impl System {
    fn connect() -> (System, Cancel) {
        (System(SessionBuilder::new().with_data_path(DATA).build().expect("opening chDB"), None), Cancel)
    }

    fn run(&mut self, sql: &str, timeout: u32) -> Result<Vec<u8>, String> {
        let csv = [Arg::OutputFormat(OutputFormat::CSV)];
        if self.1 != Some(timeout) {
            let seconds = if timeout > 0 { timeout as f64 / 1000.0 + 1.0 } else { 0.0 };
            self.0.execute(&format!("SET max_execution_time = {seconds}"), Some(&csv)).map_err(|e| e.to_string())?;
            self.1 = Some(timeout);
        }
        Ok(self.0.execute(sql, Some(&csv)).map_err(|e| e.to_string())?.data_ref().to_vec())
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
