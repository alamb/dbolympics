//! Demand over time: growth, seasonality, holidays, day-of-week and hour-of-day patterns.

use chrono::{Datelike, NaiveDate, Weekday};

use crate::time::{self, Day};
use crate::world::{Merchant, Vertical};

/// The day after US Thanksgiving (fourth Thursday of November).
pub fn black_friday(year: i32) -> NaiveDate {
    let mut d = NaiveDate::from_ymd_opt(year, 11, 1).unwrap();
    while d.weekday() != Weekday::Thu {
        d = d.succ_opt().unwrap();
    }
    d + chrono::Duration::days(22)
}

/// Extra capacity that carriers and warehouses add for the peak season (seasonal staff,
/// extra vans and sorting shifts).
pub fn peak_capacity(day: Day) -> f64 {
    let d = time::date_of(day);
    let bf = black_friday(d.year());
    let off = (d - bf).num_days();
    if (-14..=26).contains(&off) { 1.7 } else { 1.0 }
}

/// Smooth monthly seasonality: linear interpolation between mid-month values.
fn monthly(season: &[f64; 12], d: NaiveDate) -> f64 {
    let m = d.month0() as usize;
    let dim = days_in_month(d) as f64;
    let pos = (d.day() as f64 - 0.5) / dim; // 0..1 within the month
    if pos < 0.5 {
        let prev = season[(m + 11) % 12];
        let t = pos + 0.5;
        prev * (1.0 - t) + season[m] * t
    } else {
        let next = season[(m + 1) % 12];
        let t = pos - 0.5;
        season[m] * (1.0 - t) + next * t
    }
}

fn days_in_month(d: NaiveDate) -> u32 {
    let (y, m) = (d.year(), d.month());
    let next = if m == 12 { NaiveDate::from_ymd_opt(y + 1, 1, 1) } else { NaiveDate::from_ymd_opt(y, m + 1, 1) }.unwrap();
    (next - NaiveDate::from_ymd_opt(y, m, 1).unwrap()).num_days() as u32
}

/// Holiday multiplier for consumer demand.
fn holiday(vertical: &str, b2b: bool, d: NaiveDate) -> f64 {
    let bf = black_friday(d.year());
    let off = (d - bf).num_days();
    let giftish = matches!(vertical, "jewelry_accessories" | "beauty" | "consumer_electronics" | "books_stationery" | "home_living");
    if b2b {
        // B2B: quiet between Christmas and New Year
        return match (d.month(), d.day()) {
            (12, 24..=31) | (1, 1) => 0.25,
            _ => 1.0,
        };
    }
    let mut f: f64 = match off {
        -7..=-1 => 1.35,  // "Black Week" run-up
        0 => 3.6,         // Black Friday
        1 | 2 => 2.1,     // weekend
        3 => 3.0,         // Cyber Monday
        4..=6 => 1.4,
        _ => 1.0,
    };
    match (d.month(), d.day()) {
        (12, 1..=17) => f *= if giftish { 1.35 } else { 1.15 },
        (12, 18..=21) => f *= 1.1,
        (12, 22..=23) => f *= 0.8,
        (12, 24) => f *= 0.55,
        (12, 25) => f *= 0.4,
        (12, 26..=31) => f *= 1.25, // boxing day / sales
        (1, 1) => f *= 0.6,
        (1, 2..=10) => f *= 1.15,  // January sales
        (2, 1..=13) if matches!(vertical, "jewelry_accessories" | "beauty") => f *= 1.45,
        _ => {}
    }
    f
}

fn day_of_week(b2b: bool, d: NaiveDate) -> f64 {
    let w = d.weekday().num_days_from_monday() as usize;
    if b2b {
        [1.15, 1.15, 1.1, 1.1, 1.0, 0.18, 0.12][w]
    } else {
        [1.06, 1.04, 1.0, 0.98, 0.94, 0.9, 1.08][w]
    }
}

/// Hour-of-day weights (local time), 24 values.
pub fn hourly(b2b: bool) -> [f64; 24] {
    if b2b {
        [0.05, 0.03, 0.02, 0.02, 0.03, 0.1, 0.4, 1.2, 2.4, 2.8, 2.9, 2.7, 2.0, 2.4, 2.6, 2.5, 2.1, 1.3, 0.6, 0.35, 0.3, 0.25, 0.15, 0.08]
    } else {
        [0.6, 0.35, 0.2, 0.12, 0.1, 0.15, 0.35, 0.7, 1.1, 1.5, 1.8, 2.0, 2.2, 2.1, 2.0, 1.8, 1.7, 1.7, 1.8, 2.0, 2.05, 1.75, 1.3, 0.9]
    }
}

/// Demand of a merchant on a day relative to its end-of-window level (before seasonality).
pub fn merchant_level(m: &Merchant, growth: f64, day: Day) -> f64 {
    let date = time::date_of(day);
    let joined = time::parse_date(&m.joined_on);
    if date < joined {
        return 0.0;
    }
    if let Some(c) = &m.churned_on {
        let churned = time::parse_date(c);
        if date >= churned {
            return 0.0;
        }
    }
    let end = time::end_date();
    let years_before_end = (end - date).num_days() as f64 / 365.25;
    let mut level = libm::exp(-libm::log(1.0 + growth) * years_before_end);
    // ramp-up after onboarding (migration of existing merchants is instant)
    let since = (date - joined).num_days() as f64;
    if joined >= time::start_date() {
        level *= (0.2 + 0.8 * (since / 120.0)).min(1.0);
    }
    if let Some(c) = &m.churned_on {
        let left = (time::parse_date(c) - date).num_days() as f64;
        if left < 150.0 {
            level *= 0.3 + 0.7 * left / 150.0; // declining before churn
        }
    }
    level
}

/// Seasonal pull of a product type on a day: +1 at its peak, -1 off-season (northern
/// hemisphere; winter items peak mid-January... with the December gift season close by).
pub fn season_factor(season: i8, day: Day) -> f64 {
    if season == 0 {
        return 0.0;
    }
    let doy = time::date_of(day).ordinal() as f64;
    let winter = libm::cos(2.0 * std::f64::consts::PI * (doy - 10.0) / 365.25);
    if season == 1 { winter } else { -winter }
}

/// Seasonal x holiday x day-of-week demand multiplier.
pub fn demand(v_name: &str, v: &Vertical, b2b: bool, day: Day) -> f64 {
    let d = time::date_of(day);
    monthly(&v.seasonality, d) * holiday(v_name, b2b, d) * day_of_week(b2b, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_fridays() {
        assert_eq!(black_friday(2023), NaiveDate::from_ymd_opt(2023, 11, 24).unwrap());
        assert_eq!(black_friday(2024), NaiveDate::from_ymd_opt(2024, 11, 29).unwrap());
        assert_eq!(black_friday(2025), NaiveDate::from_ymd_opt(2025, 11, 28).unwrap());
    }
}
