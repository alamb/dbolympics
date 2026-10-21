//! Subscription plans (coffee clubs, pet food autoship, beauty and letterbox boxes) and their
//! price versions. A price change creates a new version of each plan (new plan and plan
//! price rows, the old plan is retired); subscribers move to it at their next renewal. The
//! subscriptions themselves live in the shopper chunks.

use super::MerchantSim;
use crate::engine::DHandle;
use crate::rng::Rng;
use crate::sim::stories;
use crate::tables::*;
use crate::time::{self, Day, MICROS_PER_DAY, MICROS_PER_HOUR, Ts};
use crate::value::{Rid, Row, V};

pub struct PlanVersion {
    /// effective for renewals from this time
    pub from: Ts,
    pub plan: DHandle,
    pub price: Rid,
    /// gross amount per billing period (cents)
    pub amount: i64,
}

pub struct PlanDef {
    pub name: String,
    pub code: String,
    /// billing interval, interval count and its length in days (approximately, for months)
    pub interval: &'static str,
    pub count: i64,
    pub months: u32,
    pub weeks: u32,
    /// deliveries per billing period (an annual plan ships monthly)
    pub deliveries: u32,
    /// products per delivery
    pub items: usize,
    pub versions: Vec<PlanVersion>,
    /// choice weight for new subscribers
    pub weight: f64,
}

pub struct Subs {
    pub plans: Vec<PlanDef>,
    /// metered add-on (extra bag): plan price and gross unit amount
    pub addon: Option<(Rid, i64)>,
    /// the 100% discount that marks prepaid subscription deliveries
    pub delivery_discount: Rid,
    pub receipt_tpl: Rid,
    pub failed_tpl: Rid,
    pending: Vec<(Day, f64)>,
}

impl PlanDef {
    /// The price version for a renewal at `t`.
    pub fn version_at(&self, t: Ts) -> usize {
        self.versions.iter().rposition(|v| v.from <= t).unwrap_or(0)
    }

    /// End of a billing period that starts at `start`.
    pub fn period_end(&self, start: Ts) -> Ts {
        if self.weeks > 0 {
            return start + self.weeks as i64 * 7 * MICROS_PER_DAY;
        }
        let d = chrono::DateTime::from_timestamp_micros(start).unwrap().naive_utc();
        let e = d.checked_add_months(chrono::Months::new(self.months)).unwrap();
        e.and_utc().timestamp_micros()
    }
}

impl MerchantSim {
    pub(super) fn setup_subscriptions(&mut self, day: Day, ts: Ts, rng: &mut Rng) {
        if !self.m.has_model("subscription") {
            return;
        }
        let tenant = self.tenant_rid();
        let unit = self.vertical_mid_eur * self.price_level * 100.0;
        // (name, code, interval, count, months, weeks, deliveries, items, price factor vs shelf, weight)
        let defs: Vec<(&str, &str, &'static str, i64, u32, u32, u32, usize, f64, f64)> = match self.m.vertical.as_str() {
            "food_coffee" => vec![
                ("Club - 1 bag monthly", "club-1-monthly", "month", 1, 1, 0, 1, 1, 0.9, 5.0),
                ("Club - 2 bags monthly", "club-2-monthly", "month", 1, 1, 0, 1, 2, 0.85, 3.0),
                ("Club - every 2 weeks", "club-fortnightly", "week", 2, 0, 2, 1, 1, 0.9, 2.0),
                ("Club - annual (12 deliveries)", "club-annual", "year", 1, 12, 0, 12, 1, 0.8 * 12.0, 1.5),
            ],
            "pet_supplies" => vec![
                ("Autoship every 4 weeks", "autoship-4w", "week", 4, 0, 4, 1, 1, 0.92, 5.0),
                ("Autoship every 8 weeks", "autoship-8w", "week", 8, 0, 8, 1, 2, 0.88, 2.0),
            ],
            "beauty" => vec![
                ("Beauty Box monthly", "box-monthly", "month", 1, 1, 0, 1, 3, 0.65, 4.0),
                ("Beauty Box quarterly", "box-quarterly", "quarter", 1, 3, 0, 1, 5, 0.6 * 1.6, 2.0),
            ],
            _ => vec![
                ("Letterbox monthly", "letterbox-monthly", "month", 1, 1, 0, 1, 3, 0.7, 4.0),
                ("Letterbox annual", "letterbox-annual", "year", 1, 12, 0, 12, 3, 0.62 * 12.0, 1.0),
            ],
        };
        let mut plans = Vec::new();
        for (name, code, interval, count, months, weeks, deliveries, items, f, weight) in defs {
            let amount = super::pricing::round_price(unit * items as f64 * f, &self.m.currency);
            let mut p = PlanDef { name: format!("{} {name}", self.m.name), code: code.to_string(), interval, count, months, weeks, deliveries, items, versions: Vec::new(), weight };
            let v = self.plan_version(day, ts, ts.min(time::ts_of_day(0)), &p, amount, None, rng);
            p.versions.push(v);
            plans.push(p);
        }
        let addon = if matches!(self.m.vertical.as_str(), "food_coffee" | "pet_supplies") {
            let amount = super::pricing::round_price(unit * 0.95, &self.m.currency);
            let mut r = Row::new(plans::NCOLS);
            r.set(plans::TENANT_ID, tenant)
                .set(plans::CODE, "addon-extra")
                .set(plans::NAME, if self.m.vertical == "food_coffee" { "Extra bag (add-on)" } else { "Extra pack (add-on)" })
                .set(plans::DEFAULT_INTERVAL, "month")
                .set(plans::INTERVAL_COUNT, 1)
                .set(plans::TRIAL_PERIOD_DAYS, 0)
                .set(plans::IS_ACTIVE, true)
                .set(plans::IS_PUBLIC, false)
                .set(plans::METADATA, V::Json(r#"{"usage_type":"metered","aggregate":"sum"}"#.to_string()))
                .set(plans::CREATED_AT, V::Ts(ts))
                .set(plans::UPDATED_AT, V::Ts(ts));
            let h = self.chunk.emit_deferred(plans::T, day, r);
            Some((self.plan_price(day, ts, h.rid, "month", 1, amount), amount))
        } else {
            None
        };
        let delivery_discount = self.discount(day, ts, "Subscription delivery (prepaid)", 100.0);
        let tpl = |c: &mut crate::engine::Chunk, code: &str, title: &str, subject: &str, body: &str| -> Rid {
            let mut r = Row::new(notification_templates::NCOLS);
            r.set(notification_templates::TENANT_ID, tenant)
                .set(notification_templates::CODE, code.to_string())
                .set(notification_templates::NAME, title.to_string())
                .set(notification_templates::CHANNEL, "email")
                .set(notification_templates::SUBJECT, subject.to_string())
                .set(notification_templates::BODY, body.to_string())
                .set(notification_templates::LOCALE, None::<String>)
                .set(notification_templates::IS_ACTIVE, true)
                .set(notification_templates::CREATED_AT, V::Ts(ts))
                .set(notification_templates::UPDATED_AT, V::Ts(ts));
            c.emit(notification_templates::T, day, r)
        };
        let receipt_tpl = tpl(&mut self.chunk, "subscription_receipt", "Subscription receipt", "Your subscription receipt", "Thanks {{first_name}}! We charged {{amount}} for {{plan}} ({{period}}).");
        let failed_tpl = tpl(&mut self.chunk, "payment_failed", "Payment failed", "Action needed: your payment failed", "We couldn't charge your card for {{plan}}. We'll try again on {{next_attempt}} - please update your payment details.");
        let pending = stories::subscription_price_changes(self.m).into_iter().map(|(d, f)| (time::day_of(d) as Day, f)).collect();
        self.subs = Some(Subs { plans, addon, delivery_discount, receipt_tpl, failed_tpl, pending });
    }

    fn plan_price(&mut self, day: Day, ts: Ts, plan: Rid, interval: &str, count: i64, amount: i64) -> Rid {
        let mut r = Row::new(plan_prices::NCOLS);
        r.set(plan_prices::TENANT_ID, self.tenant_rid())
            .set(plan_prices::PLAN_ID, plan)
            .set(plan_prices::CURRENCY_CODE, self.m.currency.clone())
            .set(plan_prices::BILLING_INTERVAL, interval.to_string())
            .set(plan_prices::INTERVAL_COUNT, count)
            .set(plan_prices::AMOUNT, V::Money(amount))
            .set(plan_prices::IS_ACTIVE, true)
            .set(plan_prices::CREATED_AT, V::Ts(ts))
            .set(plan_prices::UPDATED_AT, V::Ts(ts));
        self.chunk.emit(plan_prices::T, day, r)
    }

    #[allow(clippy::too_many_arguments)]
    fn plan_version(&mut self, day: Day, ts: Ts, from: Ts, p: &PlanDef, amount: i64, year: Option<i32>, rng: &mut Rng) -> PlanVersion {
        let tenant = self.tenant_rid();
        let code = match year { Some(y) => format!("{}-{y}", p.code), None => p.code.clone() };
        let mut r = Row::new(plans::NCOLS);
        r.set(plans::TENANT_ID, tenant)
            .set(plans::CODE, code)
            .set(plans::NAME, p.name.clone())
            .set(plans::DESCRIPTION, if rng.chance(0.7) { V::Str(format!("{} product(s) per delivery, free shipping, skip or cancel any time.", p.items)) } else { V::Null })
            .set(plans::DEFAULT_INTERVAL, p.interval)
            .set(plans::INTERVAL_COUNT, p.count)
            .set(plans::TRIAL_PERIOD_DAYS, 0)
            .set(plans::IS_ACTIVE, true)
            .set(plans::IS_PUBLIC, true)
            .set(plans::METADATA, V::Json(format!(r#"{{"deliveries_per_period":{},"items_per_delivery":{},"version":{}}}"#, p.deliveries, p.items, year.map_or("1".to_string(), |y| y.to_string()))))
            .set(plans::CREATED_AT, V::Ts(ts))
            .set(plans::UPDATED_AT, V::Ts(ts));
        let h = self.chunk.emit_deferred(plans::T, day, r);
        let price = self.plan_price(day, ts, h.rid, p.interval, p.count, amount);
        for (key, limit, on) in [("free_shipping", None, true), ("skip_deliveries", Some(3), true), ("member_discount_pct", Some(10), p.deliveries == 1), ("gift_wrapping", None, false)] {
            let mut r = Row::new(plan_features::NCOLS);
            r.set(plan_features::TENANT_ID, tenant)
                .set(plan_features::PLAN_ID, h.rid)
                .set(plan_features::FEATURE_KEY, key)
                .set(plan_features::LIMIT_VALUE, limit.map_or(V::Null, |x: i64| V::Int(x)))
                .set(plan_features::IS_ENABLED, on)
                .set(plan_features::CREATED_AT, V::Ts(ts))
                .set(plan_features::UPDATED_AT, V::Ts(ts));
            self.chunk.emit(plan_features::T, day, r);
        }
        PlanVersion { from, plan: h, price, amount }
    }

    /// Price changes are announced 30 days ahead with new plan versions.
    pub(super) fn subscriptions_day(&mut self, day: Day, rng: &mut Rng) {
        let Some(subs) = self.subs.as_mut() else { return };
        let due: Vec<(Day, f64)> = subs.pending.iter().copied().filter(|(d, _)| *d == day + 30).collect();
        for (d, f) in due {
            let ts = time::ts_of_day(day) + 9 * MICROS_PER_HOUR;
            let from = time::ts_of_day(d);
            let year = { use chrono::Datelike; time::date_of(d).year() };
            let n = self.subs.as_ref().unwrap().plans.len();
            for i in 0..n {
                let (amount, old) = {
                    let p = &self.subs.as_ref().unwrap().plans[i];
                    let last = p.versions.last().unwrap();
                    (super::pricing::round_price(last.amount as f64 * f, &self.m.currency), last.plan)
                };
                let p = std::mem::replace(&mut self.subs.as_mut().unwrap().plans[i], PlanDef { name: String::new(), code: String::new(), interval: "month", count: 1, months: 1, weeks: 0, deliveries: 1, items: 1, versions: Vec::new(), weight: 0.0 });
                let v = self.plan_version(day, ts, from, &p, amount, Some(year), rng);
                self.subs.as_mut().unwrap().plans[i] = p;
                self.subs.as_mut().unwrap().plans[i].versions.push(v);
                // the old version is closed to new subscribers when the new one takes effect
                self.chunk.deferred_mut(old).set(plans::IS_ACTIVE, false).set(plans::IS_PUBLIC, false).set(plans::UPDATED_AT, V::Ts(from));
            }
        }
    }
}
