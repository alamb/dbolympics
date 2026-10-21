//! The PostgreSQL protocol (feature `pg`), for the systems that speak it: one connection for the
//! run, the simple query protocol (values as text), results as CSV, and a cancel that waits until
//! the system has taken it (as libpq).

use std::io::Write as _;
use std::path::Path;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::{TcpStream, UnixStream};
use tokio::runtime::{Builder, Runtime};
use tokio_postgres::config::Host;
use tokio_postgres::{CancelToken, Client, Config, NoTls};

pub use tokio_postgres::SimpleQueryMessage;

/// A connection to the system: `connect` is a libpq connection string, e.g.
/// "host=/run/dbo port=5432 user=postgres dbname=postgres".
pub struct Pg {
    rt: Runtime,
    client: Client,
    config: Config,
}

impl Pg {
    /// Connect, again until the system takes connections (`crate::retry`).
    pub fn connect(connect: &str) -> Pg {
        let config: Config = connect.parse().expect("a connection string");
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let client = crate::retry(|| {
            rt.block_on(async {
                let (client, connection) = config.connect(NoTls).await?;
                // the connection's messages, handled while a statement runs (block_on)
                tokio::spawn(connection);
                Ok::<_, tokio_postgres::Error>(client)
            })
        });
        Pg { rt, client, config }
    }

    /// Run a statement: its messages, or the system's error message.
    pub fn query(&mut self, sql: &str) -> Result<Vec<SimpleQueryMessage>, String> {
        self.rt.block_on(self.client.simple_query(sql)).map_err(|e| e.as_db_error().map_or(e.to_string(), |d| d.message().to_string()))
    }

    /// Run a statement: the rows of its result as CSV.
    pub fn run(&mut self, sql: &str) -> Result<Vec<u8>, String> {
        self.query(sql).map(|m| csv(&m))
    }

    /// What another thread needs to cancel the running statement.
    pub fn canceller(&self) -> PgCancel {
        PgCancel { token: self.client.cancel_token(), config: self.config.clone() }
    }
}

/// Cancels the running statement of a `Pg`, from another thread.
pub struct PgCancel {
    token: CancelToken,
    config: Config,
}

impl PgCancel {
    /// The cancel request on a new connection; then wait until the system closes that connection,
    /// as libpq does: the system has taken the cancel by then. Without the wait, a statement that
    /// ends just at its deadline could leave the cancel to reach the next statement.
    pub fn cancel(&self) {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let port = self.config.get_ports().first().copied().unwrap_or(5432);
        let r = rt.block_on(async {
            match self.config.get_hosts().first() {
                Some(Host::Unix(dir)) => request(UnixStream::connect(Path::new(dir).join(format!(".s.PGSQL.{port}"))).await?, &self.token).await,
                Some(Host::Tcp(host)) => request(TcpStream::connect((host.as_str(), port)).await?, &self.token).await,
                None => Err(std::io::Error::other("no host")),
            }
        });
        if let Err(e) = r {
            let _ = writeln!(std::io::stderr(), "cancel: {e}");
        }
    }
}

/// Send the cancel request over `stream`, and read until the system closes it.
async fn request<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, token: &CancelToken) -> std::io::Result<()> {
    token.cancel_query_raw(&mut stream, NoTls).await.map_err(std::io::Error::other)?;
    stream.read_to_end(&mut Vec::new()).await.map(|_| ())
}

/// The rows of a statement's result (values as text), as CSV: NULL as nothing, the empty string
/// as "".
pub fn csv(messages: &[SimpleQueryMessage]) -> Vec<u8> {
    let mut out = Vec::new();
    for m in messages {
        if let SimpleQueryMessage::Row(row) = m {
            for i in 0..row.len() {
                if i > 0 {
                    out.push(b',');
                }
                crate::csv_field(&mut out, row.get(i).map(str::as_bytes));
            }
            out.push(b'\n');
        }
    }
    out
}

/// The first column of every row, one line each (e.g. the query an EXPLAIN wrote).
pub fn lines(messages: &[SimpleQueryMessage]) -> String {
    let rows: Vec<&str> = messages.iter().filter_map(|m| if let SimpleQueryMessage::Row(r) = m { Some(r.get(0).unwrap_or("")) } else { None }).collect();
    rows.join("\n")
}
