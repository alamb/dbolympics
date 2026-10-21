//! What every query program shares: the runner's protocol (systems/README.md) on
//! /run/dbo/dbo.sock, served one statement at a time, with a timer that cancels a statement at
//! its deadline; and two helpers, `retry` and `csv_field`. With the feature `pg`, `pg` has the
//! PostgreSQL protocol.
//!
//! A request is `u32` length, `u32` timeout in ms (0: none), then the SQL; the length counts the
//! timeout and the SQL. An answer is `u32` length, `u8` status (0 ok, 1 error, 2 timeout), then
//! the result as CSV or the error's text; the length counts the status and what follows. All
//! integers little endian.

#[cfg(feature = "pg")]
pub mod pg;

use std::io::{BufReader, Read, Write};
use std::os::unix::net::UnixListener;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

pub const SOCKET: &str = "/run/dbo/dbo.sock";

/// Serve the runner's protocol, for the whole run: `run` executes a statement (its SQL, its
/// timeout in ms) and gives its result as CSV; at the deadline `cancel` is called from another
/// thread, and whatever the statement does then is a timeout.
pub fn serve(mut run: impl FnMut(&str, u32) -> Result<Vec<u8>, String>, cancel: impl Fn() + Sync) {
    let _ = std::fs::remove_file(SOCKET);
    for stream in UnixListener::bind(SOCKET).unwrap().incoming() {
        let mut w = stream.unwrap();
        let mut r = BufReader::new(w.try_clone().unwrap());
        let mut head = [0u8; 8];
        while r.read_exact(&mut head).is_ok() {
            let length = u32::from_le_bytes(head[..4].try_into().unwrap()) as usize;
            let timeout = u32::from_le_bytes(head[4..].try_into().unwrap());
            let mut sql = vec![0; length - 4];
            r.read_exact(&mut sql).unwrap();
            // the timer: it ends when the statement does, or cancels it at the deadline
            let (done, wait) = mpsc::channel::<()>();
            let (result, late) = thread::scope(|s| {
                let cancel = &cancel;
                let timer = s.spawn(move || {
                    let late = timeout > 0 && wait.recv_timeout(Duration::from_millis(timeout.into())) == Err(RecvTimeoutError::Timeout);
                    if late {
                        cancel();
                    }
                    late
                });
                let result = run(&String::from_utf8_lossy(&sql), timeout);
                drop(done);
                (result, timer.join().unwrap())
            });
            let (status, out) = match result {
                _ if late => (2, Vec::new()),
                Ok(out) => (0, out),
                Err(e) => (1, e.into_bytes()),
            };
            let mut answer = Vec::with_capacity(out.len() + 5);
            answer.extend((out.len() as u32 + 1).to_le_bytes());
            answer.push(status);
            answer.extend(out);
            w.write_all(&answer).unwrap();
        }
    }
}

/// `connect` again until it succeeds (the system may still be starting); each new error once, for
/// query.log.
pub fn retry<T, E: std::fmt::Display>(mut connect: impl FnMut() -> Result<T, E>) -> T {
    let mut last = String::new();
    loop {
        match connect() {
            Ok(c) => return c,
            Err(e) if e.to_string() != last => {
                last = e.to_string();
                eprintln!("{last}");
            }
            Err(_) => {}
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// Append a CSV field: NULL as nothing, the empty string as "", quoted when it must be.
pub fn csv_field(out: &mut Vec<u8>, v: Option<&[u8]>) {
    let Some(v) = v else { return };
    if v.is_empty() || v.iter().any(|c| matches!(c, b',' | b'"' | b'\n' | b'\r')) {
        out.push(b'"');
        for &c in v {
            if c == b'"' {
                out.push(b'"');
            }
            out.push(c);
        }
        out.push(b'"');
    } else {
        out.extend(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_fields() {
        let mut out = Vec::new();
        for v in [None, Some(&b""[..]), Some(b"a"), Some(b"a,b"), Some(b"say \"hi\"")] {
            csv_field(&mut out, v);
            out.push(b'|');
        }
        assert_eq!(out, b"|\"\"|a|\"a,b\"|\"say \"\"hi\"\"\"|");
    }
}
