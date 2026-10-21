//! Compare query results by value, not by text.
//!
//! Results are CSV without a header, one row per line, compared in order (the queries are
//! deterministic). Two values are equal when:
//! - they are identical, or both NULL (empty or `NULL`; CSV cannot tell NULL from '');
//! - both are booleans with the same value (`t`/`true`, `f`/`false`, any case);
//! - both are integers and equal (exactly, so counts cannot slip by one);
//! - both are numbers and close, in the numpy form with the expected value as the reference:
//!   |actual - expected| <= ATOL + RTOL * |expected|, RTOL = ATOL = 1e-4. The queries do not
//!   round (docs/NUMERICS.md), so this absorbs engine-defined arithmetic: double vs decimal,
//!   summation order, the scale an engine gives a quotient;
//! - an exact rational `x/y*2^p` (Umbra's `debug.semana.exactarithmetic`; `/y` and `*2^p` are
//!   optional, all integers of any length) counts as the number it denotes;
//! - both are timestamps that differ only in trailing fractional zeros (`.411140` vs `.41114`);
//! - both are JSON documents that are equal once parsed (key order and spacing do not matter).
//!
//! Deliberate differences to the Python tool: integers are compared exactly (not as doubles),
//! and an empty line is one empty field (Python's csv module reads it as a row with no fields).

pub type Row = Vec<String>;

/// Parse CSV (RFC 4180; `\n` or `\r\n` line ends). Like Python's csv module, text after a
/// closing quote is appended to the field.
pub fn parse_csv(text: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut at_line_start = true;
    while let Some(c) = chars.next() {
        at_line_start = false;
        match c {
            '"' if field.is_empty() => {
                while let Some(c) = chars.next() {
                    if c == '"' {
                        if chars.peek() == Some(&'"') {
                            chars.next();
                            field.push('"');
                        } else {
                            break;
                        }
                    } else {
                        field.push(c);
                    }
                }
            }
            ',' => row.push(std::mem::take(&mut field)),
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                at_line_start = true;
            }
            c => field.push(c),
        }
    }
    if !at_line_start {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn is_int(s: &str) -> bool {
    let d = s.strip_prefix(['+', '-']).unwrap_or(s);
    !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit())
}

/// `[-+]?(\d+(\.\d*)?|\.\d+)([eE][-+]?\d+)?`
fn is_num(s: &str) -> bool {
    let b = s.strip_prefix(['+', '-']).unwrap_or(s).as_bytes();
    let digits = |i: usize| b[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = digits(0);
    if i > 0 {
        if b.get(i) == Some(&b'.') {
            i += 1 + digits(i + 1);
        }
    } else if b.first() == Some(&b'.') && b.len() > 1 && b[1].is_ascii_digit() {
        i = 1 + digits(1);
    } else {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(b.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let d = if j < b.len() { digits(j) } else { 0 };
        if d == 0 {
            return false;
        }
        i = j + d;
    }
    i == b.len()
}

/// The value of an exact rational `[-]x[/y][*2^[-]p]` (x, y of any length), or None.
fn rational(s: &str) -> Option<f64> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (frac, p) = match s.split_once("*2^") {
        Some((f, p)) => (f, p.parse::<i64>().ok()?),
        None => (s, 0),
    };
    let (x, y) = frac.split_once('/').unwrap_or((frac, "1"));
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit());
    if !digits(x) || !digits(y) || (frac == s && p == 0 && !s.contains('/')) {
        return None; // a plain integer is not a rational here
    }
    // log10 of an integer of any length: leading 17 digits plus the number of the rest
    let log10 = |t: &str| -> f64 {
        let t = t.trim_start_matches('0');
        if t.is_empty() {
            return f64::NEG_INFINITY;
        }
        let k = t.len().min(17);
        t[..k].parse::<f64>().unwrap().log10() + (t.len() - k) as f64
    };
    let l = log10(x) - log10(y) + p as f64 * std::f64::consts::LOG10_2;
    let v = if l == f64::NEG_INFINITY { 0.0 } else { 10f64.powf(l) };
    Some(if neg { -v } else { v })
}

/// A number: decimal, scientific or exact rational.
fn number(s: &str) -> Option<f64> {
    if is_num(s) { s.parse().ok() } else { rational(s) }
}

pub const RTOL: f64 = 1e-4;
pub const ATOL: f64 = 1e-4;

/// numpy.isclose form, with `expected` as the reference.
fn close(expected: f64, actual: f64) -> bool {
    (actual - expected).abs() <= ATOL + RTOL * expected.abs()
}

/// Integer value with sign and leading zeros normalized.
fn int_norm(s: &str) -> (bool, &str) {
    let (neg, d) = match s.as_bytes()[0] {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    let d = d.trim_start_matches('0');
    (neg && !d.is_empty(), d)
}

/// (date and time with 'T' as ' ', fraction without trailing zeros, zone) of a timestamp
/// `YYYY-MM-DD[ T]HH:MM:SS[.f+][(+|-)HH[[:]MM]]`.
fn timestamp(s: &str) -> Option<(String, &str, &str)> {
    let b = s.as_bytes();
    let pat = b"dddd-dd-dd_dd:dd:dd";
    if b.len() < pat.len() {
        return None;
    }
    for (c, p) in b.iter().zip(pat) {
        let ok = match p {
            b'd' => c.is_ascii_digit(),
            b'_' => *c == b' ' || *c == b'T',
            p => c == p,
        };
        if !ok {
            return None;
        }
    }
    let mut i = pat.len();
    let mut frac = "";
    if b.get(i) == Some(&b'.') {
        let n = b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        if n == 0 {
            return None;
        }
        frac = &s[i..i + 1 + n];
        i += 1 + n;
    }
    let zone = &s[i..];
    if !zone.is_empty() {
        let z = zone.as_bytes();
        let dd = |k: usize| z.len() >= k + 2 && z[k].is_ascii_digit() && z[k + 1].is_ascii_digit();
        let ok = (z[0] == b'+' || z[0] == b'-')
            && dd(1)
            && (z.len() == 3 || (z.len() == 5 && dd(3)) || (z.len() == 6 && z[3] == b':' && dd(4)));
        if !ok {
            return None;
        }
    }
    let frac = frac.trim_end_matches('0').trim_end_matches('.');
    Some((s[..pat.len()].replace('T', " "), frac, zone))
}

fn json(s: &str) -> Option<serde_json::Value> {
    let s = s.trim();
    if !s.starts_with(['[', '{']) {
        return None;
    }
    serde_json::from_str(s).ok()
}

fn json_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value::*;
    match (a, b) {
        (Object(x), Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| json_equal(v, w))),
        (Array(x), Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(v, w)| json_equal(v, w)),
        (Bool(x), Bool(y)) => x == y,
        (Number(x), Number(y)) if x.is_i64() && y.is_i64() => x.as_i64() == y.as_i64(),
        (Number(x), Number(y)) => close(x.as_f64().unwrap_or(f64::NAN), y.as_f64().unwrap_or(f64::NAN)),
        _ => a == b,
    }
}

/// Is `b` (actual) equal to `a` (expected)?
pub fn values_equal(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let null = |s: &str| s.is_empty() || s == "NULL";
    if null(a) && null(b) {
        return true;
    }
    let (la, lb) = (a.to_ascii_lowercase(), b.to_ascii_lowercase());
    let truthy = |s: &str| s == "t" || s == "true";
    let falsy = |s: &str| s == "f" || s == "false";
    if (truthy(&la) && truthy(&lb)) || (falsy(&la) && falsy(&lb)) {
        return true;
    }
    if is_int(a) && is_int(b) {
        return int_norm(a) == int_norm(b);
    }
    if let (Some(x), Some(y)) = (number(a), number(b)) {
        return close(x, y);
    }
    if let (Some(ta), Some(tb)) = (timestamp(a), timestamp(b)) {
        return ta == tb;
    }
    if let (Some(ja), Some(jb)) = (json(a), json(b)) {
        return json_equal(&ja, &jb);
    }
    false
}

/// The differences between two results (at most `limit`); empty when they match.
pub fn compare(expected: &[Row], actual: &[Row], limit: usize) -> Vec<String> {
    let mut diffs = Vec::new();
    if expected.len() != actual.len() {
        diffs.push(format!("row count {} vs {}", expected.len(), actual.len()));
    }
    for (i, (r, s)) in expected.iter().zip(actual).enumerate() {
        if diffs.len() >= limit {
            break;
        }
        if r.len() != s.len() {
            diffs.push(format!("row {}: {} vs {} columns", i + 1, r.len(), s.len()));
            continue;
        }
        let bad: Vec<String> = r
            .iter()
            .zip(s)
            .enumerate()
            .filter(|(_, (u, v))| !values_equal(u, v))
            .take(3)
            .map(|(j, (u, v))| format!("col {}: {u:?} vs {v:?}", j + 1))
            .collect();
        if !bad.is_empty() {
            diffs.push(format!("row {}: {}", i + 1, bad.join("; ")));
        }
    }
    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv() {
        assert_eq!(parse_csv("a,b\n\"x,\"\"y\"\"\",\n\n"), vec![vec!["a", "b"], vec!["x,\"y\"", ""], vec![""]]);
        assert_eq!(parse_csv("a\r\nb"), vec![vec!["a"], vec!["b"]]);
        assert_eq!(parse_csv("\"multi\nline\",1\n"), vec![vec!["multi\nline", "1"]]);
        assert_eq!(parse_csv(""), Vec::<Row>::new());
        assert_eq!(parse_csv("\"ab\"c,d\n"), vec![vec!["abc", "d"]]);
    }

    #[test]
    fn nulls_and_booleans() {
        assert!(values_equal("", "NULL"));
        assert!(values_equal("t", "TRUE"));
        assert!(values_equal("False", "f"));
        assert!(!values_equal("t", "f"));
        assert!(!values_equal("", "0"));
    }

    #[test]
    fn numbers() {
        assert!(values_equal("42", "+042"));
        assert!(values_equal("-0", "0"));
        assert!(!values_equal("9007199254740993", "9007199254740992"));
        assert!(values_equal("41.860000", "41.86"));
        assert!(values_equal("1214.5", "1214.4999999999998"));
        assert!(values_equal("12.799100", "12.79913819091699"));
        assert!(values_equal("12.7991", "12.8") && !values_equal("12.7991", "12.802"));
        assert!(values_equal("0.00001", "0.00009") && !values_equal("0", "0.00011"));
        assert!(values_equal("1e6", "1000099") && !values_equal("1e6", "1000101.5"));
        assert!(!values_equal("10", "10.5"));
        assert!(values_equal("1e-12", "-3.5e-10"));
        assert!(values_equal("1.5e10", "15000000000.0"));
        assert!(!values_equal("1.5e10", "1.6e10"));
        assert!(values_equal("1", "1.0") && !values_equal("1", "1.1"));
        assert!(!is_num("1e") && !is_num(".") && !is_num("+") && is_num(".5") && is_num("5.") && is_num("-1.5E+3"));
    }

    #[test]
    fn rationals() {
        assert!((rational("1/3").unwrap() - 1.0 / 3.0).abs() < 1e-15);
        assert!(values_equal("1/3*2^0", "0.33333"));
        assert!(values_equal("12.5", "25/2") && values_equal("25/2", "12.5"));
        assert!(values_equal("3*2^-2", "0.75") && values_equal("-3/4*2^1", "-1.5"));
        assert!(!values_equal("1/3", "0.34"));
        // numerator and denominator far beyond f64's integer range
        let big = format!("{}/{}", "1".repeat(400), "3".repeat(399) + "0");
        assert!(values_equal(&big, "0.33333333"), "{:?}", rational(&big));
        assert!(values_equal("0/7", "0") && rational("12").is_none() && rational("1/x").is_none() && rational("1/2*2^").is_none());
        assert!(values_equal("4", "8/2"));
    }

    #[test]
    fn timestamps() {
        assert!(values_equal("2025-01-02 03:04:05.411140", "2025-01-02T03:04:05.41114"));
        assert!(values_equal("2025-01-02 03:04:05.000", "2025-01-02 03:04:05"));
        assert!(values_equal("2025-01-02 03:04:05+01", "2025-01-02 03:04:05.0+01"));
        assert!(!values_equal("2025-01-02 03:04:05+01", "2025-01-02 03:04:05+02"));
        assert!(!values_equal("2025-01-02 03:04:05.1", "2025-01-02 03:04:05.2"));
        assert!(timestamp("2025-01-02 03:04:05+0100").is_some() && timestamp("2025-01-02 03:04:05+01:00").is_some());
        assert!(timestamp("2025-01-02 03:04:05.").is_none() && timestamp("2025-01-02 03:04:05Z").is_none());
    }

    #[test]
    fn json_values() {
        assert!(values_equal(r#"{"a": 1, "b": [true, null]}"#, r#"{"b":[true,null],"a":1.0}"#));
        assert!(!values_equal(r#"{"a": 1}"#, r#"{"a": 2}"#));
        assert!(!values_equal(r#"{"a": true}"#, r#"{"a": 1}"#));
        assert!(!values_equal(r#"{"a": 100000}"#, r#"{"a": 100001}"#) && values_equal(r#"{"a": 100000.0}"#, r#"{"a": 100001.0}"#));
        assert!(!values_equal(r#"{"a": 1}"#, r#"{"a": 1, "b": 2}"#));
        assert!(!values_equal("[1", "[1 "));
    }

    #[test]
    fn compare_rows() {
        let r = |rows: &[&[&str]]| rows.iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect::<Vec<Row>>();
        assert!(compare(&r(&[&["1", "x"]]), &r(&[&["1", "x"]]), 5).is_empty());
        assert_eq!(compare(&r(&[&["1"]]), &r(&[&["1"], &["2"]]), 5), vec!["row count 1 vs 2"]);
        assert_eq!(compare(&r(&[&["1", "2"]]), &r(&[&["1"]]), 5), vec!["row 1: 2 vs 1 columns"]);
        assert_eq!(compare(&r(&[&["a", "b"]]), &r(&[&["a", "c"]]), 5), vec!["row 1: col 2: \"b\" vs \"c\""]);
    }
}
