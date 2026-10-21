//! Hyper's query program: one connection to hyperd (start) over the PostgreSQL protocol, for the
//! whole run (Hyper closes a database when its last connection ends, and opens it again cold).
//! It serves the runner's protocol through ../dbo (dbo::pg: the PostgreSQL protocol).

/// where the system listens (a socket's directory, or an address), who connects
const CONNECT: &str = "host=127.0.0.1 port=7483 user=postgres dbname=/system/db/dbo.hyper";

fn main() {
    let mut pg = dbo::pg::Pg::connect(CONNECT);
    let cancel = pg.canceller();
    dbo::serve(|sql, _timeout| pg.run(sql), || cancel.cancel());
}
