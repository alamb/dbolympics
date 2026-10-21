//! Numbers in reference results: every number (a decimal, scientific notation, or an exact
//! rational `x/y*2^p` from Umbra's `debug.semana.exactarithmetic`) is written with at most 40
//! decimals, computed exactly. A value that is exact within 40 decimals has its trailing zeros
//! trimmed (`41.860000` -> `41.86`, `1/8` -> `0.125`); a value that needs rounding is rounded
//! half to even and keeps all 40 decimals, so a rounded value is recognizable (a tiny value
//! that rounds to zero is `0.` and 40 zeros). Integers stay as they are.

use num_bigint::BigUint;
use num_integer::Integer;

pub const DECIMALS: u32 = 40;

/// The exact value of a number field as (negative, numerator, denominator), or None for
/// integers and anything that is not a number.
fn parse(s: &str) -> Option<(bool, BigUint, BigUint)> {
    let (neg, t) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits = |u: &str| !u.is_empty() && u.bytes().all(|c| c.is_ascii_digit());
    let big = |u: &str| u.parse::<BigUint>().ok();
    let pow = |b: u32, e: u32| BigUint::from(b).pow(e);
    // exact rational: x[/y][*2^p], at least one of /y and *2^p
    if t.contains('/') || t.contains("*2^") {
        let (frac, p) = match t.split_once("*2^") {
            Some((f, p)) => (f, p.parse::<i64>().ok()?),
            None => (t, 0),
        };
        let (x, y) = frac.split_once('/').unwrap_or((frac, "1"));
        if !digits(x) || !digits(y) {
            return None;
        }
        let (mut n, mut d) = (big(x)?, big(y)?);
        if d == BigUint::from(0u32) {
            return None;
        }
        if p >= 0 {
            n *= pow(2, p as u32);
        } else {
            d *= pow(2, (-p) as u32);
        }
        return Some((neg, n, d));
    }
    // decimal or scientific: d[.d][e[+-]k], with a '.' or an exponent (integers stay)
    let (mant, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], t[i + 1..].parse::<i64>().ok()?),
        None => (t, 0),
    };
    if !mant.contains('.') && exp == 0 && !t.contains(['e', 'E']) {
        return None;
    }
    let (ip, fp) = mant.split_once('.').unwrap_or((mant, ""));
    if (ip.is_empty() && fp.is_empty()) || !(ip.is_empty() || digits(ip)) || !(fp.is_empty() || digits(fp)) {
        return None;
    }
    let mut n = big(&format!("{ip}{fp}"))?;
    let mut d = pow(10, fp.len() as u32);
    if exp >= 0 {
        n *= pow(10, exp as u32);
    } else {
        d *= pow(10, (-exp) as u32);
    }
    Some((neg, n, d))
}

/// A number field written with at most 40 decimals (see the module documentation); None when
/// the field is not converted (integers, strings).
pub fn to_decimal(s: &str) -> Option<String> {
    let (neg, n, d) = parse(s)?;
    let scale = BigUint::from(10u32).pow(DECIMALS);
    let (mut q, r) = (n * &scale).div_rem(&d);
    let exact = r == BigUint::from(0u32);
    if !exact {
        let twice = r << 1usize;
        if twice > d || (twice == d && q.is_odd()) {
            q += 1u32;
        }
    }
    let (int, frac) = q.div_rem(&scale);
    let mut f = frac.to_string();
    f = "0".repeat(DECIMALS as usize - f.len()) + &f;
    if exact {
        f.truncate(f.trim_end_matches('0').len());
    }
    let zero = int == BigUint::from(0u32) && f.bytes().all(|c| c == b'0');
    // an exact zero has no sign; a rounded nonzero value keeps it
    let sign = if neg && !(exact && zero) { "-" } else { "" };
    Some(if f.is_empty() { format!("{sign}{int}") } else { format!("{sign}{int}.{f}") })
}

/// Convert every unquoted number field of a CSV result (quoted fields are strings and stay as
/// they are, so NULL and '' stay distinct).
pub fn convert_csv(csv: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(csv.len());
    let mut i = 0;
    while i < csv.len() {
        if csv[i] == b'"' {
            // a quoted field, copied up to its closing quote
            let start = i;
            i += 1;
            while i < csv.len() {
                if csv[i] == b'"' {
                    if csv.get(i + 1) == Some(&b'"') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.extend_from_slice(&csv[start..i]);
            continue;
        }
        let end = csv[i..].iter().position(|&c| c == b',' || c == b'\n').map_or(csv.len(), |k| i + k);
        let field = &csv[i..end];
        match std::str::from_utf8(field).ok().and_then(to_decimal) {
            Some(v) => out.extend_from_slice(v.as_bytes()),
            None => out.extend_from_slice(field),
        }
        if end < csv.len() {
            out.push(csv[end]);
        }
        i = end + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_values_are_trimmed() {
        assert_eq!(to_decimal("41.860000").as_deref(), Some("41.86"));
        assert_eq!(to_decimal("1/8").as_deref(), Some("0.125"));
        assert_eq!(to_decimal("1/7*2^1").as_deref(), Some(&*format!("0.{}", "2857142857142857142857142857142857142857")));
        assert_eq!(to_decimal("3*2^-2").as_deref(), Some("0.75"));
        assert_eq!(to_decimal("2.000").as_deref(), Some("2"));
        assert_eq!(to_decimal("-0.50").as_deref(), Some("-0.5"));
        assert_eq!(to_decimal("1.5e3").as_deref(), Some("1500"));
        assert_eq!(to_decimal("2.5E-3").as_deref(), Some("0.0025"));
        assert_eq!(to_decimal("0/9").as_deref(), Some("0"));
        assert_eq!(to_decimal("-0.0").as_deref(), Some("0"));
    }

    #[test]
    fn rounded_values_keep_40_decimals() {
        assert_eq!(to_decimal("1/3").unwrap(), format!("0.{}", "3".repeat(40)));
        assert_eq!(to_decimal("2/3").unwrap(), format!("0.{}7", "6".repeat(39)));
        // rounds to zero: all 40 zeros stay
        let tiny = format!("0.{}1", "0".repeat(45));
        assert_eq!(to_decimal(&tiny).unwrap(), format!("0.{}", "0".repeat(40)));
        assert_eq!(to_decimal(&format!("-{tiny}")).unwrap(), format!("-0.{}", "0".repeat(40)));
        // half to even at the 41st decimal
        assert_eq!(to_decimal(&format!("0.{}5", "0".repeat(40))).unwrap(), format!("0.{}", "0".repeat(40)));
        assert_eq!(to_decimal(&format!("0.{}15", "0".repeat(39))).unwrap(), format!("0.{}2", "0".repeat(39)));
        // a numerator far beyond 64 bits
        let r = to_decimal(&format!("{}/{}", "1".repeat(300), "3".repeat(299))).unwrap();
        assert_eq!(r.len(), "3.".len() + 40, "{r}");
    }

    #[test]
    fn integers_and_strings_stay() {
        for s in ["42", "-7", "0010", "abc", "2025-01-02", "2025-01-02 03:04:05.5", "1/x", "1.2.3", "", "."] {
            assert_eq!(to_decimal(s), None, "{s}");
        }
    }

    #[test]
    fn csv_fields() {
        let csv = b"1,41.860000,\"1/8\",,\"\",1/8\n2,\"a,0.50\",3*2^-2,x\n";
        assert_eq!(String::from_utf8(convert_csv(csv)).unwrap(), "1,41.86,\"1/8\",,\"\",0.125\n2,\"a,0.50\",0.75,x\n");
    }
}
