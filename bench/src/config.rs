//! A benchmark file (`bench/benchmark_*.toml`): what to run, on which systems (default: all),
//! with which system configurations. See docs/RUNNER.md.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Which keys the schema declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Keys {
    None,
    Primary,
}

impl Keys {
    pub fn name(self) -> &'static str {
        match self {
            Keys::None => "none",
            Keys::Primary => "primary",
        }
    }
}

/// A system configuration: every system of the benchmark runs once per configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// names the result file (`systems/<system>/results/<date>/<name>.csv`)
    pub name: String,
    pub sf: f64,
    pub threads: u32,
    /// memory limit of the system, e.g. "100GB" (binary units: 1 GB = 2^30 bytes)
    pub memory: String,
    /// override the benchmark's (after `read`, always set: the benchmark's if not given here)
    pub query_timeout: Option<u64>,
    pub load_timeout: Option<u64>,
    pub grace_period: Option<u64>,
    /// the cores to pin to, e.g. "0-7" (default: the first `threads` cores)
    pub cpuset: Option<String>,
    /// overrides the benchmark's `keys`
    pub keys: Option<Keys>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Benchmark {
    /// directories below `systems` (default: every one with a system.toml)
    pub systems: Option<Vec<String>>,
    /// runs of all queries after every load
    pub repetitions: u32,
    /// seconds per query
    pub query_timeout: u64,
    /// seconds for each load, all its statements together (default: none); a load over it fails
    /// the run at once, without a grace period (`failed_load_timeout`)
    pub load_timeout: Option<u64>,
    /// seconds the runner waits past a query's timeout for the system's answer (the cancel)
    /// before the run fails as stuck (`failed_stuck`)
    pub grace_period: u64,
    #[serde(default = "default_keys")]
    pub keys: Keys,
    /// a subset of the queries, e.g. `["q001-q020", "q100"]` (default: all)
    pub queries: Option<Vec<String>>,
    #[serde(rename = "config")]
    pub configs: Vec<Config>,
}

fn default_keys() -> Keys {
    Keys::None
}

/// The periods of a dataset in time order, one data file per table each (as in the generator,
/// src/time.rs): everything through 2024, then the quarters of 2025. Every period is one load.
pub fn periods() -> Vec<String> {
    let mut v = vec!["2024-and-earlier".to_string()];
    v.extend((1..=4).map(|q| format!("2025-Q{q}")));
    v
}

/// A size like "100GB", "512MB" or a number of bytes.
pub fn parse_size(s: &str) -> Result<u64> {
    let t = s.trim();
    let split = t.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(t.len());
    let n: f64 = t[..split].parse().with_context(|| format!("size {s:?}"))?;
    let unit: u64 = match t[split..].trim().to_ascii_uppercase().as_str() {
        "" | "B" => 1,
        "K" | "KB" | "KIB" => 1 << 10,
        "M" | "MB" | "MIB" => 1 << 20,
        "G" | "GB" | "GIB" => 1 << 30,
        "T" | "TB" | "TIB" => 1 << 40,
        u => bail!("size {s:?}: unknown unit {u:?}"),
    };
    Ok((n * unit as f64) as u64)
}

pub fn sf_name(sf: f64) -> String {
    let s = format!("{sf}");
    s.strip_suffix(".0").map(String::from).unwrap_or(s)
}

impl Config {
    pub fn memory_bytes(&self) -> Result<u64> {
        parse_size(&self.memory).with_context(|| format!("config {}: memory", self.name))
    }

    /// seconds per query
    pub fn query_timeout(&self) -> u64 {
        self.query_timeout.expect("set by Benchmark::read")
    }

    /// seconds past a query's timeout before the run fails as stuck
    pub fn grace_period(&self) -> u64 {
        self.grace_period.expect("set by Benchmark::read")
    }

    pub fn cpuset(&self) -> String {
        self.cpuset.clone().unwrap_or_else(|| if self.threads == 1 { "0".into() } else { format!("0-{}", self.threads - 1) })
    }
}

impl Benchmark {
    pub fn read(path: &Path) -> Result<Benchmark> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut b: Benchmark = toml::from_str(&text).with_context(|| format!("benchmark file {}", path.display()))?;
        b.resolve()?;
        Ok(b)
    }

    /// Each configuration's timeouts and grace period: its own, else the benchmark's.
    fn resolve(&mut self) -> Result<()> {
        let b = self;
        if b.repetitions == 0 {
            bail!("repetitions must be at least 1");
        }
        for c in &mut b.configs {
            c.query_timeout = c.query_timeout.or(Some(b.query_timeout));
            c.load_timeout = c.load_timeout.or(b.load_timeout);
            c.grace_period = c.grace_period.or(Some(b.grace_period));
        }
        for (i, c) in b.configs.iter().enumerate() {
            c.memory_bytes()?;
            if c.threads == 0 || c.query_timeout == Some(0) || c.load_timeout == Some(0) {
                bail!("config {}: threads, query_timeout and load_timeout must be at least 1", c.name);
            }
            if !c.name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.') || c.name.is_empty() {
                bail!("config name {:?}: letters, digits, '-', '_' and '.' only", c.name);
            }
            if b.configs[..i].iter().any(|d| d.name == c.name) {
                bail!("config {} is defined twice", c.name);
            }
        }
        Ok(())
    }

    /// The selected queries among `ids` (all sorted query ids), in order.
    pub fn select_queries(&self, ids: &[String]) -> Result<Vec<String>> {
        let Some(sel) = &self.queries else { return Ok(ids.to_vec()) };
        let mut keep = vec![false; ids.len()];
        for s in sel {
            let (a, b) = s.split_once('-').unwrap_or((s, s));
            let (i, j) = (ids.iter().position(|q| q == a), ids.iter().position(|q| q == b));
            match (i, j) {
                (Some(i), Some(j)) if i <= j => keep[i..=j].iter_mut().for_each(|k| *k = true),
                _ => bail!("queries: {s:?} does not name queries"),
            }
        }
        Ok(ids.iter().zip(keep).filter(|(_, k)| *k).map(|(q, _)| q.clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(parse_size("100GB").unwrap(), 100 << 30);
        assert_eq!(parse_size("512 MB").unwrap(), 512 << 20);
        assert_eq!(parse_size("1.5g").unwrap(), 3 << 29);
        assert_eq!(parse_size("4096").unwrap(), 4096);
        assert!(parse_size("1 parsec").is_err());
    }

    #[test]
    fn queries() {
        let text = "systems = [\"duckdb\"]\nrepetitions = 3\nquery_timeout = 60\ngrace_period = 60\nqueries = [\"q002-q003\", \"q005\"]\n[[config]]\nname = \"a\"\nsf = 1\nthreads = 4\nmemory = \"8GB\"\n";
        let mut b: Benchmark = toml::from_str(text).unwrap();
        b.resolve().unwrap();
        assert_eq!(periods(), vec!["2024-and-earlier", "2025-Q1", "2025-Q2", "2025-Q3", "2025-Q4"]);
        let ids: Vec<String> = (1..=6).map(|i| format!("q{i:03}")).collect();
        assert_eq!(b.select_queries(&ids).unwrap(), vec!["q002", "q003", "q005"]);
        assert_eq!((b.keys, b.configs[0].cpuset()), (Keys::None, "0-3".to_string()));
        assert_eq!((b.configs[0].query_timeout(), b.configs[0].load_timeout, b.configs[0].grace_period()), (60, None, 60));
        // a configuration's own values win
        let mut b: Benchmark = toml::from_str(&format!("{text}query_timeout = 5\nload_timeout = 600\n")).unwrap();
        b.resolve().unwrap();
        assert_eq!((b.configs[0].query_timeout(), b.configs[0].load_timeout, b.configs[0].grace_period()), (5, Some(600), 60));
    }
}
