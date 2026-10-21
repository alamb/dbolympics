//! DuckDB's query program: DuckDB embedded, the database open for the whole run. It serves the
//! runner's protocol (../dbo). A result is written by Arrow's CSV
//! writer; a timeout interrupts the running statement.

use std::sync::Arc;

use duckdb::{Connection, InterruptHandle};

// --- The system ----------------------------------------------------------------------------------

const DATABASE: &str = "/system/db/dbo.duckdb";

struct System(Connection);
struct Cancel(Arc<InterruptHandle>);

impl System {
    fn connect() -> (System, Cancel) {
        std::fs::create_dir_all("/system/db").unwrap();
        let conn = Connection::open(DATABASE).expect("opening the database");
        let cancel = Cancel(conn.interrupt_handle());
        (System(conn), cancel)
    }

    fn run(&mut self, sql: &str, _timeout: u32) -> Result<Vec<u8>, String> {
        let e = |e: duckdb::Error| e.to_string();
        let mut stmt = self.0.prepare(sql).map_err(e)?;
        let mut out = Vec::new();
        let mut w = arrow_csv::WriterBuilder::new().with_header(false).build(&mut out);
        for b in stmt.query_arrow([]).map_err(e)? {
            w.write(&b).map_err(|e| e.to_string())?;
        }
        drop(w);
        Ok(out)
    }
}

impl Cancel {
    fn cancel(&self) {
        self.0.interrupt();
    }
}

// --- The runner's protocol (../dbo) -------------------------------------------------------------

fn main() {
    let (mut system, cancel) = System::connect();
    dbo::serve(|sql, timeout| system.run(sql, timeout), || cancel.cancel());
}
