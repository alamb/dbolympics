//! Umbra's query program: one connection to Umbra (its socket, in /run/dbo), the simple query
//! protocol (values as text).
//! It serves the runner's protocol through ../dbo (dbo::pg: the PostgreSQL protocol).

/// where the system listens (a socket's directory, or an address), who connects
const CONNECT: &str = "host=/run/dbo port=5432 user=postgres password=postgres dbname=postgres";

fn main() {
    let mut pg = dbo::pg::Pg::connect(CONNECT);
    let cancel = pg.canceller();
    dbo::serve(|sql, _timeout| pg.run(sql), || cancel.cancel());
}
