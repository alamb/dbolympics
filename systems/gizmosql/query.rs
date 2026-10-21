//! GizmoSQL's query program: Arrow Flight SQL, one session (its bearer token) for the run. It
//! serves the runner's protocol (../dbo). A result is written by
//! Arrow's CSV writer; a timeout drops the running call.

use std::sync::Arc;

use arrow_flight::sql::client::FlightSqlServiceClient;
use futures::TryStreamExt;
use tokio::runtime::Runtime;
use tokio::sync::Notify;
use tonic::transport::{Channel, Endpoint};

// --- The system ----------------------------------------------------------------------------------

/// its address, the user of start
const URI: &str = "http://127.0.0.1:31337";
const USER: &str = "bench";
const PASSWORD: &str = "bench";

struct System {
    rt: Runtime,
    client: FlightSqlServiceClient<Channel>,
    stop: Arc<Notify>,
}
struct Cancel(Arc<Notify>);

impl System {
    fn connect() -> (System, Cancel) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let client = dbo::retry(|| {
            rt.block_on(async {
                let mut c = FlightSqlServiceClient::new(Endpoint::from_static(URI).connect().await.map_err(|e| e.to_string())?);
                c.handshake(USER, PASSWORD).await.map_err(|e| e.to_string())?;
                Ok::<_, String>(c)
            })
        });
        let stop = Arc::new(Notify::new());
        (System { rt, client, stop: stop.clone() }, Cancel(stop))
    }

    fn run(&mut self, sql: &str, _timeout: u32) -> Result<Vec<u8>, String> {
        let client = &mut self.client;
        let run = async {
            let info = client.execute(sql.to_string(), None).await.map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            let mut w = arrow_csv::WriterBuilder::new().with_header(false).build(&mut out);
            for ep in info.endpoint {
                let ticket = ep.ticket.ok_or("an endpoint without a ticket")?;
                let mut stream = client.do_get(ticket).await.map_err(|e| e.to_string())?;
                while let Some(b) = stream.try_next().await.map_err(|e| e.to_string())? {
                    w.write(&b).map_err(|e| e.to_string())?;
                }
            }
            drop(w);
            Ok(out)
        };
        self.rt.block_on(async {
            tokio::select! {
                r = run => r,
                _ = self.stop.notified() => Err("cancelled".into()),
            }
        })
    }
}

impl Cancel {
    /// notify_waiters keeps nothing for later: a cancel after the end cannot reach the next statement
    fn cancel(&self) {
        self.0.notify_waiters();
    }
}

// --- The runner's protocol (../dbo) -------------------------------------------------------------

fn main() {
    let (mut system, cancel) = System::connect();
    dbo::serve(|sql, timeout| system.run(sql, timeout), || cancel.cancel());
}
