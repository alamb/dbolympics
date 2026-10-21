//! Simulation calendar and timestamps. Day 0 = 2023-01-01; timestamps are microseconds since
//! the Unix epoch, UTC, stored as `timestamp` (without time zone).

use chrono::{Datelike, NaiveDate, Weekday};

pub const START: (i32, u32, u32) = (2023, 1, 1);
pub const END: (i32, u32, u32) = (2025, 12, 31);
pub const MICROS_PER_SEC: i64 = 1_000_000;
pub const MICROS_PER_MIN: i64 = 60 * MICROS_PER_SEC;
pub const MICROS_PER_HOUR: i64 = 60 * MICROS_PER_MIN;
pub const MICROS_PER_DAY: i64 = 24 * MICROS_PER_HOUR;

pub type Day = u32;
pub type Ts = i64;

pub fn start_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(START.0, START.1, START.2).unwrap()
}

pub fn end_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(END.0, END.1, END.2).unwrap()
}

/// Number of simulated days (day indices 0..num_days()).
pub fn num_days() -> Day {
    (end_date() - start_date()).num_days() as Day + 1
}

pub fn date_of(day: Day) -> NaiveDate {
    start_date() + chrono::Duration::days(day as i64)
}

/// Day index of a date; dates before the start map to negative values.
pub fn day_of(d: NaiveDate) -> i64 {
    (d - start_date()).num_days()
}

pub fn parse_date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or_else(|_| panic!("bad date {s}"))
}

/// Days since 1970-01-01 (Arrow Date32).
pub fn epoch_days(d: NaiveDate) -> i32 {
    (d - NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()).num_days() as i32
}

/// Midnight UTC of a date.
pub fn ts_of_date(d: NaiveDate) -> Ts {
    epoch_days(d) as i64 * MICROS_PER_DAY
}

pub fn ts_of_day(day: Day) -> Ts {
    ts_of_date(date_of(day))
}

/// Simulation day of a timestamp (may be negative before the start).
pub fn day_of_ts(ts: Ts) -> i64 {
    ts.div_euclid(MICROS_PER_DAY) - epoch_days(start_date()) as i64
}

pub fn end_ts() -> Ts {
    ts_of_date(end_date()) + MICROS_PER_DAY - 1
}

// --------------------------------------------------------------------------------------
// Periods: the data files of every table
// --------------------------------------------------------------------------------------

/// The file of the rows from before the last year: those that existed before the 2023 migration,
/// and those of 2023 and 2024. The benchmark loads it first. The name sorts before the quarters,
/// so the files of a table sort in time order.
pub const BEFORE: &str = "2024-and-earlier";

/// The names of the periods in time order: `BEFORE`, then one per quarter of the last year
/// ("2025-Q1" ...). Every table has one data file per period.
pub fn periods() -> Vec<String> {
    let mut v = vec![BEFORE.to_string()];
    v.extend((1..=4).map(|q| format!("{}-Q{q}", END.0)));
    v
}

/// Index into `periods()` of a simulated day: `BEFORE` before the last year, else its quarter.
pub fn period_of_day(day: Day) -> usize {
    let d = date_of(day);
    if d.year() < END.0 { 0 } else { 1 + d.month0() as usize / 3 }
}

// --------------------------------------------------------------------------------------
// Time zones (enough for the merchant and shopper countries)
// --------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DstRule {
    None,
    /// EU/UK: last Sunday of March 01:00 UTC to last Sunday of October 01:00 UTC.
    Eu,
    /// US: second Sunday of March 02:00 local to first Sunday of November 02:00 local.
    Us,
}

#[derive(Clone, Copy, Debug)]
pub struct Tz {
    pub std_offset_min: i32,
    pub dst: DstRule,
}

pub fn tz_by_name(name: &str) -> Tz {
    let eu = |h: i32| Tz { std_offset_min: h * 60, dst: DstRule::Eu };
    let us = |h: i32| Tz { std_offset_min: h * 60, dst: DstRule::Us };
    match name {
        "Europe/London" | "Europe/Dublin" | "Europe/Lisbon" => eu(0),
        "Europe/Helsinki" | "Europe/Athens" | "Europe/Bucharest" | "Europe/Tallinn" | "Europe/Riga" | "Europe/Vilnius" | "Europe/Sofia" => eu(2),
        n if n.starts_with("Europe/") => eu(1),
        "America/New_York" | "America/Toronto" => us(-5),
        "America/Chicago" => us(-6),
        "America/Denver" => us(-7),
        "America/Phoenix" => Tz { std_offset_min: -7 * 60, dst: DstRule::None },
        "America/Los_Angeles" | "America/Vancouver" => us(-8),
        "Australia/Sydney" => Tz { std_offset_min: 10 * 60, dst: DstRule::None },
        "Asia/Tokyo" => Tz { std_offset_min: 9 * 60, dst: DstRule::None },
        "Asia/Singapore" | "Asia/Hong_Kong" | "Asia/Shanghai" => Tz { std_offset_min: 8 * 60, dst: DstRule::None },
        "Asia/Kolkata" => Tz { std_offset_min: 330, dst: DstRule::None },
        "Asia/Dubai" => Tz { std_offset_min: 4 * 60, dst: DstRule::None },
        "America/Sao_Paulo" => Tz { std_offset_min: -3 * 60, dst: DstRule::None },
        "America/Mexico_City" => Tz { std_offset_min: -6 * 60, dst: DstRule::None },
        "Africa/Johannesburg" => Tz { std_offset_min: 2 * 60, dst: DstRule::None },
        "Pacific/Auckland" => Tz { std_offset_min: 12 * 60, dst: DstRule::None },
        "UTC" => Tz { std_offset_min: 0, dst: DstRule::None },
        _ => panic!("unknown time zone {name}"),
    }
}

fn last_sunday(y: i32, m: u32) -> NaiveDate {
    let next = if m == 12 { NaiveDate::from_ymd_opt(y + 1, 1, 1) } else { NaiveDate::from_ymd_opt(y, m + 1, 1) }.unwrap();
    let mut d = next.pred_opt().unwrap();
    while d.weekday() != Weekday::Sun {
        d = d.pred_opt().unwrap();
    }
    d
}

fn nth_sunday(y: i32, m: u32, n: u32) -> NaiveDate {
    let mut d = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
    while d.weekday() != Weekday::Sun {
        d = d.succ_opt().unwrap();
    }
    d + chrono::Duration::days(7 * (n as i64 - 1))
}

impl Tz {
    /// UTC offset in minutes at a UTC instant.
    pub fn offset_at(&self, utc: Ts) -> i32 {
        let base = self.std_offset_min;
        let year = chrono::DateTime::from_timestamp_micros(utc).unwrap().year();
        let in_dst = match self.dst {
            DstRule::None => false,
            DstRule::Eu => {
                let s = ts_of_date(last_sunday(year, 3)) + MICROS_PER_HOUR;
                let e = ts_of_date(last_sunday(year, 10)) + MICROS_PER_HOUR;
                utc >= s && utc < e
            }
            DstRule::Us => {
                let s = ts_of_date(nth_sunday(year, 3, 2)) + 2 * MICROS_PER_HOUR - base as i64 * MICROS_PER_MIN;
                let e = ts_of_date(nth_sunday(year, 11, 1)) + 2 * MICROS_PER_HOUR - (base + 60) as i64 * MICROS_PER_MIN;
                utc >= s && utc < e
            }
        };
        base + if in_dst { 60 } else { 0 }
    }

    /// UTC instant of a local wall-clock time (local midnight of `date` + `micros_into_day`).
    pub fn local_to_utc(&self, date: NaiveDate, micros_into_day: i64) -> Ts {
        let local = ts_of_date(date) + micros_into_day;
        let guess = local - self.std_offset_min as i64 * MICROS_PER_MIN;
        local - self.offset_at(guess) as i64 * MICROS_PER_MIN
    }

    pub fn utc_to_local(&self, utc: Ts) -> Ts {
        utc + self.offset_at(utc) as i64 * MICROS_PER_MIN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar() {
        assert_eq!(num_days(), 1096);
        assert_eq!(date_of(0), start_date());
        assert_eq!(day_of_ts(ts_of_day(17) + 5), 17);
        assert_eq!(day_of_ts(ts_of_day(0) - 1), -1);
        let p = periods();
        assert_eq!((p.len(), p[0].as_str(), p[1].as_str(), p[4].as_str()), (5, BEFORE, "2025-Q1", "2025-Q4"));
        assert!(p.windows(2).all(|w| w[0] < w[1]));
        assert_eq!((period_of_day(0), period_of_day(730), period_of_day(731), period_of_day(num_days() - 1)), (0, 0, 1, 4));
    }

    #[test]
    fn dst() {
        let london = tz_by_name("Europe/London");
        let jan = ts_of_date(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
        let jul = ts_of_date(NaiveDate::from_ymd_opt(2025, 7, 15).unwrap());
        assert_eq!(london.offset_at(jan), 0);
        assert_eq!(london.offset_at(jul), 60);
        let ny = tz_by_name("America/New_York");
        assert_eq!(ny.offset_at(jan), -300);
        assert_eq!(ny.offset_at(jul), -240);
        let berlin = tz_by_name("Europe/Berlin");
        let d = NaiveDate::from_ymd_opt(2025, 7, 15).unwrap();
        assert_eq!(berlin.local_to_utc(d, 12 * MICROS_PER_HOUR), ts_of_date(d) + 10 * MICROS_PER_HOUR);
    }
}
