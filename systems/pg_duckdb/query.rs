//! pg_duckdb's query program: one connection to PostgreSQL (its socket, in /run/dbo), the simple
//! query protocol (values as text).
//! It serves the runner's protocol through ../dbo (dbo::pg: the PostgreSQL protocol).

/// where the system listens (a socket's directory, or an address), who connects
const CONNECT: &str = "host=/run/dbo port=5432 user=postgres password=Dbolympics-Bench-2026! dbname=postgres";

fn main() {
    let mut pg = dbo::pg::Pg::connect(CONNECT);
    let cancel = pg.canceller();
    dbo::serve(|sql, _timeout| pg.run(sql), || cancel.cancel());
}
