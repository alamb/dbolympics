//! The planted stories (docs/STORIES.md) as changes to the world's fundamentals. Kept in one
//! place so the rest of the simulator stays story-agnostic: everything else asks these
//! functions, and the effects (delays, tickets, churn, revenue) emerge downstream.

use chrono::Datelike;

use crate::time::{self, Day, MICROS_PER_HOUR, MICROS_PER_MIN, Ts};
use crate::world::Merchant;

pub fn has(m: &Merchant, story: &str) -> bool {
    m.stories.iter().any(|s| s == story)
}

fn ts(y: i32, mo: u32, d: u32, h: i64, mi: i64) -> Ts {
    time::ts_of_date(chrono::NaiveDate::from_ymd_opt(y, mo, d).unwrap()) + h * MICROS_PER_HOUR + mi * MICROS_PER_MIN
}

fn day(y: i32, mo: u32, d: u32) -> Day {
    time::day_of(chrono::NaiveDate::from_ymd_opt(y, mo, d).unwrap()) as Day
}

/// S1: Albion Parcel's capacity crisis (2025-11-10 to 2025-12-08, worst in Black Friday
/// week). Capacity factor of a carrier network on a day (1.0 = normal).
pub fn carrier_capacity(carrier: &str, d: Day) -> f64 {
    let base = crate::sim::calendar::peak_capacity(d);
    if carrier == "Albion Parcel" && d >= day(2025, 11, 10) && d <= day(2025, 12, 8) {
        let worst = d >= day(2025, 11, 24) && d <= day(2025, 12, 1);
        return base * if worst { 0.25 } else { 0.42 };
    }
    // Kennet Freight added capacity for the merchants that moved over from Albion
    if carrier == "Kennet Freight" && d >= day(2025, 11, 24) && d <= day(2026, 1, 31) {
        return base * 3.5;
    }
    base
}

/// S1: Marrowby & Vane moves its UK parcels from Albion Parcel to Kennet Freight.
pub fn carrier_override(m: &Merchant, carrier: &str, at: Ts) -> Option<&'static str> {
    (has(m, "S1") && carrier == "Albion Parcel" && at >= ts(2025, 11, 24, 0, 0)).then_some("Kennet Freight")
}

/// S2: Halyard Pay degraded on 2025-03-14 06:10-14:40 UTC.
pub fn gateway_down(gateway: &str, at: Ts) -> bool {
    gateway == "Halyard Pay" && at >= ts(2025, 3, 14, 6, 10) && at < ts(2025, 3, 14, 14, 40)
}

/// S2: while it recovered, some charges were captured twice.
pub fn gateway_recovering(gateway: &str, at: Ts) -> bool {
    gateway == "Halyard Pay" && at >= ts(2025, 3, 14, 13, 30) && at < ts(2025, 3, 14, 15, 30)
}

/// S3: the platform degraded on Black Friday 2025, 14:00-15:50 UTC.
pub fn platform_down(at: Ts) -> bool {
    at >= ts(2025, 11, 28, 14, 0) && at < ts(2025, 11, 28, 15, 50)
}

/// S4: from July 2025 the electronics merchant's main supplier delivers in ~45 days instead
/// of ~14 (actual lead time in days).
pub fn s4_lead_days(m: &Merchant, d: Day) -> Option<f64> {
    if !has(m, "S4") || d < day(2025, 7, 1) {
        None
    } else if d < day(2025, 8, 1) {
        Some(30.0)
    } else {
        Some(45.0)
    }
}

pub fn s4_start(d: Day) -> bool {
    d == day(2025, 7, 1)
}

/// S4: in October the merchant moves its best sellers to the second supplier.
pub fn s4_switch(m: &Merchant, d: Day) -> bool {
    has(m, "S4") && d == day(2025, 10, 6)
}

/// S5: the first stand-mixer lot received in spring 2025 is defective; recalled in May.
pub fn s5_defect_window(m: &Merchant, d: Day) -> bool {
    has(m, "S5") && d >= day(2025, 3, 3) && d <= day(2025, 4, 30)
}

pub fn s5_start(m: &Merchant, d: Day) -> bool {
    has(m, "S5") && d == day(2025, 3, 3)
}

pub fn s5_recall(m: &Merchant, d: Day) -> bool {
    has(m, "S5") && d == day(2025, 5, 19)
}

/// Subscription price changes of a merchant: (effective date, factor). S6: the largest coffee
/// subscription merchant raises prices ~18% on 2025-04-01; about half of the others make a
/// small inflation adjustment in spring 2024.
pub fn subscription_price_changes(m: &Merchant) -> Vec<(chrono::NaiveDate, f64)> {
    let mut v = Vec::new();
    let h = crate::rng::hash_str(&format!("{}|sub-price", m.slug));
    if h % 2 == 0 {
        v.push((chrono::NaiveDate::from_ymd_opt(2024, 3 + (h / 2 % 4) as u32, 1).unwrap(), 1.05 + (h / 8 % 4) as f64 * 0.01));
    }
    if has(m, "S6") {
        v.retain(|(d, _)| d.year() != 2025);
        v.push((chrono::NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(), 1.18));
    }
    v
}

/// S6: the win-back email to cancelled coffee subscribers (an A/B test of subject lines).
pub fn s6_winback(m: &Merchant, d: Day) -> bool {
    has(m, "S6") && d == day(2025, 6, 16)
}

/// S7: the influencer "Spring Mega Sale" (40% off with a code), 2025-05-05 to 2025-05-25.
pub fn s7_sale_start(m: &Merchant, d: chrono::NaiveDate) -> bool {
    has(m, "S7") && d == chrono::NaiveDate::from_ymd_opt(2025, 5, 5).unwrap()
}

pub fn s7_window(m: &Merchant, d: Day) -> bool {
    has(m, "S7") && d >= day(2025, 5, 5) && d <= day(2025, 5, 25)
}

pub fn s7_cohort_day(m: &Merchant, d: Day) -> bool {
    has(m, "S7") && d == day(2025, 5, 27)
}

/// S8: the coupon-fraud ring at a beauty merchant (Aug-Oct 2025), shut down on 2025-10-27.
pub fn s8_window(m: &Merchant, d: Day) -> bool {
    has(m, "S8") && d >= day(2025, 8, 4) && d <= day(2025, 10, 24)
}

pub fn s8_crackdown(m: &Merchant, d: Day) -> bool {
    has(m, "S8") && d == day(2025, 10, 27)
}

/// S9: two support agents leave the B2B wholesaler on 2025-08-01; replacements are trained
/// in October. Support capacity factor.
pub fn s9_leave(m: &Merchant) -> Option<Ts> {
    has(m, "S9").then(|| ts(2025, 8, 1, 0, 0))
}

pub fn support_capacity(m: &Merchant, d: Day) -> f64 {
    if !has(m, "S9") || d < day(2025, 8, 1) {
        1.0
    } else if d < day(2025, 10, 1) {
        0.5
    } else if d < day(2025, 11, 1) {
        0.75
    } else {
        1.0
    }
}

/// S10: the sports merchant's mobile app launches on 2025-05-15.
pub fn s10_launch(m: &Merchant) -> Option<Ts> {
    has(m, "S10").then(|| ts(2025, 5, 15, 8, 0))
}

/// S11: the checkout experiment whose assignment favours returning users for the treatment.
pub fn s11_biased(m: &Merchant, key: &str) -> bool {
    has(m, "S11") && key == "checkout_express_pay"
}

/// S12: the analytics SDK version a visitor's browser runs (4.2.0 double-sends events on
/// mobile web between 2025-06-02 and 2025-06-10).
pub fn sdk_version(d: Day, mobile_web: bool) -> (&'static str, bool) {
    if d < day(2025, 6, 2) {
        ("3.8.1", false)
    } else if d <= day(2025, 6, 10) {
        if mobile_web { ("4.2.0", true) } else { ("3.8.1", false) }
    } else {
        ("4.2.1", false)
    }
}

/// S13: orders that reach the home merchant through its ERP integration; from the DST change
/// on 2024-10-27 their timestamps are local Berlin time written as UTC.
pub fn s13_erp(m: &Merchant) -> bool {
    has(m, "S13")
}

pub fn s13_bug(at: Ts) -> bool {
    at >= ts(2024, 10, 27, 1, 0)
}

/// S14: the pet merchant re-imports its customer list on 2024-02-12.
pub fn s14_import(m: &Merchant, d: Day) -> bool {
    has(m, "S14") && d == day(2024, 2, 12)
}

/// S16: credential stuffing against shopper logins, 2025-02-08 to 2025-02-11.
pub fn s16_attack(m: &Merchant, d: Day) -> bool {
    has(m, "S16") && d >= day(2025, 2, 8) && d <= day(2025, 2, 11)
}

pub fn s16_response(m: &Merchant, d: Day) -> bool {
    has(m, "S16") && d == day(2025, 2, 12)
}

/// S18: the gift guide season at the home merchant (Nov-Dec 2024).
pub fn s18_gift_season(m: &Merchant, d: Day) -> bool {
    has(m, "S18") && d >= day(2024, 11, 1) && d <= day(2024, 12, 24)
}

/// S19: products onboarded from 2025 on at the fast-growing apparel merchant.
pub fn s19_sloppy(m: &Merchant, d: Day) -> bool {
    has(m, "S19") && d >= day(2025, 1, 1)
}

/// S19: the apparel merchant restructures its category tree on 2025-03-03.
pub fn s19_restructure(m: &Merchant, d: Day) -> bool {
    has(m, "S19") && d == day(2025, 3, 3)
}

/// S20: the office wholesaler's 2025: a competitor undercuts it in H1, the new volume price
/// list arrives in June, H2 recovers at lower margin.
pub fn s20_slump(m: &Merchant, d: Day) -> bool {
    has(m, "S20") && d >= day(2025, 1, 1) && d < day(2025, 7, 1)
}

pub fn s20_price_list(m: &Merchant, d: Day) -> bool {
    has(m, "S20") && d == day(2025, 6, 2)
}

pub fn s20_winback(m: &Merchant, d: Day) -> bool {
    has(m, "S20") && d == day(2025, 9, 15)
}
