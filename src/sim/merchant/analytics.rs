//! The merchant's analytics layer: daily metric rollups and monthly KPI snapshots (computed
//! by nightly jobs from what happened, so they inherit sampling and data-quality issues),
//! signup cohorts, funnel definitions, and server-side experiments.

use super::{DayOut, MerchantSim};
use crate::engine::DHandle;
use crate::rng::Rng;
use crate::tables::*;
use crate::time::{self, Day, MICROS_PER_DAY, MICROS_PER_HOUR, MICROS_PER_MIN, Ts};
use crate::value::{Rid, Row, V};

/// What shopper chunks report at the start of a month about the previous one.
#[derive(Default, Clone, Copy)]
pub struct MonthKpi {
    pub active: i64,
    pub repeat: i64,
    /// monthly recurring revenue of active subscriptions (cents)
    pub mrr: i64,
    /// cohort members added for the cohort created today
    pub cohort: i64,
}

pub struct Experiment {
    pub rid: Rid,
    pub key: String,
    pub start: Ts,
    pub end: Ts,
    /// (variant, key, conversion multiplier)
    pub variants: Vec<(Rid, &'static str, f64)>,
}

#[derive(Default)]
pub struct Analytics {
    pub enabled: bool,
    month: [i64; 3],
    prev: std::collections::BTreeMap<&'static str, i64>,
    /// today's cohort (for the chunks to fill): (cohort, members signed up in [from, to),
    /// only the influencer campaign's audience)
    pub cohort_today: Option<(Rid, Ts, Ts, bool)>,
    pending_cohort: Option<DHandle>,
    pub experiments: Vec<Experiment>,
}

impl MerchantSim {
    pub(super) fn setup_analytics(&mut self, day: Day, ts: Ts, rng: &mut Rng) {
        let tenant = self.tenant_rid();
        self.analytics.enabled = self.m.size_rank <= 40;
        if self.analytics.enabled {
            for (name, window, steps) in [
                ("Checkout funnel", 1, &[("Viewed product", "product_viewed"), ("Added to cart", "add_to_cart"), ("Started checkout", "checkout_started"), ("Completed order", "order_completed")][..]),
                ("Search to purchase", 7, &[("Searched", "search"), ("Viewed product", "product_viewed"), ("Added to cart", "add_to_cart"), ("Completed order", "order_completed")][..]),
                ("Signup", 30, &[("Viewed product", "product_viewed"), ("Signed up", "signup")][..]),
            ] {
                let mut r = Row::new(funnels::NCOLS);
                r.set(funnels::TENANT_ID, tenant).set(funnels::NAME, name).set(funnels::IS_ACTIVE, true).set(funnels::WINDOW_DAYS, window).set(funnels::CREATED_AT, V::Ts(ts)).set(funnels::UPDATED_AT, V::Ts(ts));
                let f = self.chunk.emit(funnels::T, day, r);
                for (i, (n, ev)) in steps.iter().enumerate() {
                    let mut r = Row::new(funnel_steps::NCOLS);
                    r.set(funnel_steps::TENANT_ID, tenant).set(funnel_steps::FUNNEL_ID, f).set(funnel_steps::STEP_ORDER, i).set(funnel_steps::NAME, *n).set(funnel_steps::EVENT_NAME, *ev).set(funnel_steps::FILTERS, V::Json(if *ev == "order_completed" && rng.chance(0.3) { r#"{"order.revenue":{"gt":0}}"#.to_string() } else { "{}".to_string() })).set(funnel_steps::CREATED_AT, V::Ts(ts)).set(funnel_steps::UPDATED_AT, V::Ts(ts));
                    self.chunk.emit(funnel_steps::T, day, r);
                }
            }
        }
        // the large merchants run a few storefront experiments a year
        if self.m.size_rank <= 12 {
            let defs = [
                ("2023-05-08", "pdp_layout_2023", "Product page layout", "A larger gallery increases add-to-cart"),
                ("2023-10-02", "free_shipping_banner", "Free shipping banner", "Showing the threshold in the header raises order value"),
                ("2024-03-04", "checkout_one_page", "One-page checkout", "Fewer checkout steps increase conversion"),
                ("2024-09-09", "reco_widget_v2", "Recommendations v2", "Bought-together recommendations increase basket size"),
                ("2025-02-03", "search_ranking", "Search ranking", "Popularity-weighted ranking improves search conversion"),
                ("2025-09-01", "checkout_express_pay", "Express pay buttons", "Wallet buttons on the cart page increase conversion"),
            ];
            for (start, key, name, hyp) in defs {
                let s = time::ts_of_date(time::parse_date(start)) + (self.idx as i64 % 7) * MICROS_PER_DAY;
                let e = s + 28 * MICROS_PER_DAY;
                if s < self.joined_ts() {
                    continue;
                }
                let done = e <= time::end_ts();
                let mut r = Row::new(experiments::NCOLS);
                r.set(experiments::TENANT_ID, tenant)
                    .set(experiments::KEY, key)
                    .set(experiments::NAME, name)
                    .set(experiments::HYPOTHESIS, hyp)
                    .set(experiments::STATUS, if done { "completed" } else { "running" })
                    .set(experiments::PRIMARY_METRIC, "conversion_rate")
                    .set(experiments::STARTS_AT, V::Ts(s))
                    .set(experiments::ENDS_AT, V::Ts(e))
                    .set(experiments::CREATED_AT, V::Ts(s - 7 * MICROS_PER_DAY))
                    .set(experiments::UPDATED_AT, V::Ts(if done { e } else { s }));
                let d = (time::day_of_ts(s - 7 * MICROS_PER_DAY).max(day as i64)) as Day;
                let rid = self.chunk.emit(experiments::T, d, r);
                let mut variants = Vec::new();
                for (k, (vk, vn)) in [("control", "Control"), ("treatment", "Treatment")].iter().enumerate() {
                    let lift = if k == 0 { 1.0 } else { rng.lognormal(0.01, 0.04) };
                    let lift = if crate::sim::stories::s11_biased(self.m, key) { 1.0 } else { lift };
                    let mut r = Row::new(experiment_variants::NCOLS);
                    r.set(experiment_variants::TENANT_ID, tenant)
                        .set(experiment_variants::EXPERIMENT_ID, rid)
                        .set(experiment_variants::KEY, *vk)
                        .set(experiment_variants::NAME, *vn)
                        .set(experiment_variants::IS_CONTROL, k == 0)
                        .set(experiment_variants::ALLOCATION_PCT, V::Dec(5000, 2))
                        .set(experiment_variants::CONFIG, V::Json(format!(r#"{{"flag":"{key}","value":{}}}"#, k == 1)))
                        .set(experiment_variants::CREATED_AT, V::Ts(s - 7 * MICROS_PER_DAY))
                        .set(experiment_variants::UPDATED_AT, V::Ts(s - 7 * MICROS_PER_DAY));
                    variants.push((self.chunk.emit(experiment_variants::T, d, r), *vk, lift));
                }
                self.analytics.experiments.push(Experiment { rid, key: key.to_string(), start: s, end: e, variants });
            }
        }
    }

    /// Nightly rollups of yesterday, monthly KPIs and cohorts.
    pub(super) fn analytics_day(&mut self, day: Day, outs: &[&DayOut]) {
        use chrono::Datelike;
        let tenant = self.tenant_rid();
        let date = time::date_of(day);
        let cur = self.m.currency.clone();
        self.analytics.cohort_today = None;
        // yesterday's cohort size
        if let Some(h) = self.analytics.pending_cohort.take() {
            let n: i64 = outs.iter().filter_map(|o| o.kpi.map(|k| k.cohort)).sum();
            self.chunk.deferred_mut(h).set(cohorts::MEMBER_COUNT, V::Int(n));
        }
        if day == 0 {
            return;
        }
        let orders: i64 = outs.iter().map(|o| o.orders as i64).sum();
        let gross: i64 = outs.iter().flat_map(|o| o.captured.iter()).filter(|c| c.amount > 0).map(|c| c.amount).sum();
        let sessions: i64 = outs.iter().map(|o| o.sessions as i64).sum();
        let events: i64 = outs.iter().map(|o| o.events as i64).sum();
        let signups: i64 = outs.iter().map(|o| o.signups as i64).sum();
        self.analytics.month[0] += orders;
        self.analytics.month[1] += gross;
        let yday = time::date_of(day - 1);
        let at = time::ts_of_day(day) + 2 * MICROS_PER_HOUR + 40 * MICROS_PER_MIN;
        if self.analytics.enabled {
            for (key, value, dims) in [
                ("orders", V::Dec(orders as i128, 0), "{}".to_string()),
                ("gross_revenue", V::Money(gross), format!(r#"{{"currency":"{cur}"}}"#)),
                ("sessions", V::Dec(sessions as i128, 0), r#"{"sampled":true,"sample_rate":0.15}"#.to_string()),
                ("events", V::Dec(events as i128, 0), r#"{"sampled":true,"sample_rate":0.15}"#.to_string()),
            ] {
                let _ = signups;
                let mut r = Row::new(metrics_daily::NCOLS);
                r.set(metrics_daily::TENANT_ID, tenant)
                    .set(metrics_daily::METRIC_KEY, key)
                    .set(metrics_daily::METRIC_DATE, V::Date(time::epoch_days(yday)))
                    .set(metrics_daily::METRIC_VALUE, value)
                    .set(metrics_daily::DIMENSIONS, V::Json(dims))
                    .set(metrics_daily::CREATED_AT, V::Ts(at))
                    .set(metrics_daily::UPDATED_AT, V::Ts(at));
                self.chunk.emit(metrics_daily::T, day, r);
            }
        }
        if date.day() == 2 {
            // monthly KPIs of the previous month (chunks reported on the 1st)
            let k = outs.iter().filter_map(|o| o.kpi).fold(MonthKpi::default(), |a, b| MonthKpi { active: a.active + b.active, repeat: a.repeat + b.repeat, mrr: a.mrr + b.mrr, cohort: 0 });
            let first = date.with_day(1).unwrap().checked_sub_months(chrono::Months::new(1)).unwrap();
            let last = date.with_day(1).unwrap() - chrono::Duration::days(1);
            let (mo, gmv) = (self.analytics.month[0], self.analytics.month[1]);
            self.analytics.month = [0; 3];
            let mut kpis: Vec<(&'static str, i64, i8)> = vec![("gmv", gmv, 2), ("orders", mo, 0), ("aov", if mo > 0 { gmv / mo } else { 0 }, 2), ("active_customers", k.active, 0), ("repeat_rate", if k.active > 0 { k.repeat * 10000 / k.active } else { 0 }, 4)];
            if self.subs.is_some() {
                kpis.push(("mrr", k.mrr, 2));
            }
            let captured = time::ts_of_day(day) + 3 * MICROS_PER_HOUR;
            for (key, v, scale) in kpis {
                let prev = self.analytics.prev.insert(key, v);
                let mut r = Row::new(kpi_snapshots::NCOLS);
                r.set(kpi_snapshots::TENANT_ID, tenant)
                    .set(kpi_snapshots::KPI_KEY, key)
                    .set(kpi_snapshots::PERIOD_GRAIN, "month")
                    .set(kpi_snapshots::PERIOD_START, V::Date(time::epoch_days(first)))
                    .set(kpi_snapshots::PERIOD_END, V::Date(time::epoch_days(last)))
                    .set(kpi_snapshots::KPI_VALUE, V::Dec(v as i128, scale))
                    .set(kpi_snapshots::PREVIOUS_VALUE, prev.map_or(V::Null, |p| V::Dec(p as i128, scale)))
                    .set(kpi_snapshots::METADATA, V::Json(if scale == 2 { format!(r#"{{"currency":"{cur}"}}"#) } else { "{}".to_string() }))
                    .set(kpi_snapshots::CAPTURED_AT, V::Ts(captured))
                    .set(kpi_snapshots::CREATED_AT, V::Ts(captured))
                    .set(kpi_snapshots::UPDATED_AT, V::Ts(captured));
                self.chunk.emit(kpi_snapshots::T, day, r);
            }
        }
        if date.day() == 1 {
            // signup cohort of the previous month, filled by the chunks today
            let first = date.checked_sub_months(chrono::Months::new(1)).unwrap();
            self.cohort(day, &format!("Signed up {}", first.format("%Y-%m")), time::ts_of_date(first), time::ts_of_date(date), false);
        } else if crate::sim::stories::s7_cohort_day(self.m, day) {
            let from = time::ts_of_date(chrono::NaiveDate::from_ymd_opt(2025, 5, 5).unwrap());
            self.cohort(day, "Spring Mega Sale 2025 - creator signups", from, time::ts_of_day(day), true);
        }
    }

    pub(super) fn migrated_cohort(&mut self, day: Day) {
        if time::parse_date(&self.m.joined_on) < time::start_date() {
            self.cohort(day, "Customers before the 2023 migration", i64::MIN / 2, time::ts_of_day(0), false);
        }
    }

    fn cohort(&mut self, day: Day, name: &str, from: Ts, to: Ts, influencer: bool) {
        let tenant = self.tenant_rid();
        let at = time::ts_of_day(day) + 4 * MICROS_PER_HOUR;
        let def = if influencer { r#"{"acquisition":{"utm_campaign":"creator_mega40"}}"#.to_string() } else if from < 0 { r#"{"signed_up_before":"2023-01-01"}"#.to_string() } else { format!(r#"{{"signed_up_from":"{}","signed_up_to":"{}"}}"#, crate::sim::shopper::iso_date(from), crate::sim::shopper::iso_date(to)) };
        let mut r = Row::new(cohorts::NCOLS);
        r.set(cohorts::TENANT_ID, tenant)
            .set(cohorts::NAME, name.to_string())
            .set(cohorts::DEFINITION, V::Json(def))
            .set(cohorts::IS_DYNAMIC, false)
            .set(cohorts::MEMBER_COUNT, V::Int(0))
            .set(cohorts::COMPUTED_AT, V::Ts(at))
            .set(cohorts::CREATED_AT, V::Ts(at))
            .set(cohorts::UPDATED_AT, V::Ts(at));
        let h = self.chunk.emit_deferred(cohorts::T, day, r);
        self.analytics.pending_cohort = Some(h);
        self.analytics.cohort_today = Some((h.rid, from, to, influencer));
    }

    /// Cohort sizes reported on the last day (no next day to read them).
    pub fn finish_analytics(&mut self, outs: &[&DayOut]) {
        if let Some(h) = self.analytics.pending_cohort.take() {
            let n: i64 = outs.iter().filter_map(|o| o.kpi.map(|k| k.cohort)).sum();
            self.chunk.deferred_mut(h).set(cohorts::MEMBER_COUNT, V::Int(n));
        }
    }
}
