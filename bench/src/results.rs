//! The result files: `systems/<system>/results/<date>/<config>.csv`, one row per run and query with
//! the median time of the repetitions and why a query has no correct result; the system, its
//! version and the date are in `<config>.json` next to it. See docs/RUNNER.md.

use std::path::Path;

use anyhow::{Context, Result};

pub const HEADER: [&str; 6] = ["run", "loaded", "query", "median_s", "status", "message"];

/// One line of a result file.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// the counter of the load: 1 is the initial load, then one per quarter
    pub run: usize,
    /// the last quarter in the database
    pub loaded: String,
    /// the query, or `load`
    pub query: String,
    /// seconds: the median over the repetitions (for `load`, the time of the load)
    pub median: Option<f64>,
    pub status: String,
    /// why there is no correct result, in the system's words; empty for `ok`
    pub message: String,
}

/// Write a CSV file: the header, then the records.
pub fn write_csv<R: IntoIterator<Item = S>, S: AsRef<[u8]>>(file: &Path, header: &[&str], records: impl IntoIterator<Item = R>) -> Result<()> {
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut w = csv::Writer::from_path(file).with_context(|| format!("writing {}", file.display()))?;
    w.write_record(header)?;
    for r in records {
        w.write_record(r)?;
    }
    w.flush().with_context(|| format!("writing {}", file.display()))
}

/// Write a system's result file (replacing an earlier one of the same day).
pub fn write(file: &Path, rows: &[Row]) -> Result<()> {
    write_csv(
        file,
        &HEADER,
        rows.iter().map(|r| [r.run.to_string(), r.loaded.clone(), r.query.clone(), r.median.map(|m| format!("{m:.6}")).unwrap_or_default(), r.status.clone(), r.message.clone()]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_file() {
        let file = std::env::temp_dir().join(format!("dbo-results-{}.csv", std::process::id()));
        let row = |q: &str, m: Option<f64>, st: &str, msg: &str| Row { run: 1, loaded: "2024-Q4".into(), query: q.into(), median: m, status: st.into(), message: msg.into() };
        write(&file, &[row("load", Some(1.5), "ok", ""), row("q001", None, "timeout", "")]).unwrap();
        write(&file, &[row("load", Some(3.0), "ok", ""), row("q002", None, "error", "column \"x\" does not exist, line 1")]).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        std::fs::remove_file(&file).unwrap();
        assert_eq!(text, "run,loaded,query,median_s,status,message\n1,2024-Q4,load,3.000000,ok,\n1,2024-Q4,q002,,error,\"column \"\"x\"\" does not exist, line 1\"\n");
    }
}
