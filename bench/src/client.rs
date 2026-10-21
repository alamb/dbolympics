//! The runner's side of the protocol every system's query program speaks (systems/README.md):
//! requests over a Unix socket, timed by the runner.

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

/// The system's answer.
pub enum Answer {
    Ok(Vec<u8>),
    Error(String),
    Timeout,
}

/// Why a request has no answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Gone {
    /// no answer within the limit
    Stuck,
    /// the connection is closed: the query program or its system is gone
    Closed,
}

pub struct Client {
    stream: UnixStream,
}

impl Client {
    pub fn connect(socket: &Path) -> std::io::Result<Client> {
        Ok(Client { stream: UnixStream::connect(socket)? })
    }

    /// Send a statement and wait for the answer, at most `limit`; `tick` is called every few
    /// seconds while waiting, and returning false gives up (Closed). The seconds are from sending
    /// the request to having the whole answer.
    pub fn request(&mut self, sql: &str, timeout: Option<Duration>, limit: Option<Duration>, tick: &mut dyn FnMut() -> bool) -> Result<(Answer, f64), Gone> {
        let ms = timeout.map_or(0, |t| t.as_millis().clamp(1, u32::MAX as u128) as u32);
        let mut req = Vec::with_capacity(sql.len() + 8);
        req.extend((sql.len() as u32 + 4).to_le_bytes());
        req.extend(ms.to_le_bytes());
        req.extend(sql.as_bytes());
        let t0 = Instant::now();
        self.stream.write_all(&req).map_err(|_| Gone::Closed)?;
        // wait for the answer to start, a few seconds at a time
        loop {
            let wait = match limit {
                Some(l) if t0.elapsed() >= l => return Err(Gone::Stuck),
                Some(l) => (l - t0.elapsed()).min(Duration::from_secs(2)),
                None => Duration::from_secs(2),
            };
            let mut p = libc::pollfd { fd: self.stream.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            let n = unsafe { libc::poll(&mut p, 1, wait.as_millis().max(1) as i32) };
            if n > 0 {
                break;
            }
            if n < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                return Err(Gone::Closed);
            }
            if !tick() {
                return Err(Gone::Closed);
            }
        }
        // the length of the rest, the status
        let mut head = [0u8; 5];
        self.stream.read_exact(&mut head).map_err(|_| Gone::Closed)?;
        let len = u32::from_le_bytes(head[..4].try_into().unwrap()) as usize;
        let mut body = vec![0u8; len.checked_sub(1).ok_or(Gone::Closed)?];
        self.stream.read_exact(&mut body).map_err(|_| Gone::Closed)?;
        let seconds = t0.elapsed().as_secs_f64();
        let answer = match head[4] {
            0 => Answer::Ok(body),
            2 => Answer::Timeout,
            _ => Answer::Error(String::from_utf8_lossy(&body).into_owned()),
        };
        Ok((answer, seconds))
    }
}
