//! CedarDB's query program: one connection to CedarDB (its own container), the simple query
//! protocol (values as text).
//! It serves the runner's protocol through ../dbo (dbo::pg: the PostgreSQL protocol).

/// where the system listens (a socket's directory, or an address), who connects
const CONNECT: &str = "host=127.0.0.1 port=5432 user=postgres password=Dbolympics-Bench-2026! dbname=postgres";

fn main() {
    let mut pg = dbo::pg::Pg::connect(CONNECT);
    let cancel = pg.canceller();
    dbo::serve(|sql, _timeout| pg.run(sql), || cancel.cancel());
}
