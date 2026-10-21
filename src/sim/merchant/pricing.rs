//! Prices over time, tax tables, discounts, coupons, promotions and B2B price rules.
//!
//! Each product has a price schedule (yearly increases, seasonal clearance, a final sale before
//! it is archived); shoppers always see the current price, so markdowns raise demand through
//! the utility model. Promotions (Black Friday, January and summer sales) are discounts at
//! checkout and also enter the utility.

use std::collections::BTreeMap;

use super::MerchantSim;
use crate::engine::DHandle;
use crate::rng::Rng;
use crate::sim::calendar;
use crate::tables::*;
use crate::time::{self, Day, MICROS_PER_DAY, MICROS_PER_HOUR, Ts};
use crate::value::{Rid, Row, V};
use crate::world::world;

/// A discount a shopper can get at checkout.
#[derive(Clone)]
pub struct Offer {
    pub discount: Rid,
    pub coupon: Option<(Rid, String)>,
    /// fraction off the line price
    pub pct: f64,
    /// None = sitewide, else the products it applies to
    pub products: Option<Vec<usize>>,
    pub start: Ts,
    pub end: Ts,
    pub name: String,
    pub promotion: Option<Rid>,
}

#[derive(Default)]
pub struct Pricing {
    pub price_list: Option<Rid>,
    pub promos: Vec<Offer>,
    pub welcome: Option<Offer>,
    /// B2B volume tiers: (min quantity, fraction off, discount)
    pub tiers: Vec<(i64, f64, Rid)>,
    /// coupon -> deferred row (times_redeemed is updated from redemptions)
    pub coupon_rows: BTreeMap<Rid, DHandle>,
    pub redemptions: BTreeMap<Rid, i64>,
}

/// Round a price to a typical ending (x.99, x.49, whole numbers for SEK/NOK/DKK).
pub fn round_price(cents: f64, currency: &str) -> i64 {
    let x = cents / 100.0;
    let p = match currency {
        "SEK" | "NOK" | "DKK" => x.round().max(1.0),
        _ if x >= 100.0 => (x / 5.0).round() * 5.0 - 0.01,
        _ => (x * 2.0).round() / 2.0 - 0.01,
    };
    (p.max(0.49) * 100.0).round() as i64
}

impl MerchantSim {
    /// Current price factor of a product (1.0 = launch price).
    pub fn price_factor(&self, pidx: usize, day: Day) -> f64 {
        let mut f = 1.0;
        for &(d, x) in &self.products[pidx].price_steps {
            if d <= day {
                f = x;
            } else {
                break;
            }
        }
        f
    }

    /// Current shelf price of a variant (gross, cents).
    pub fn price_now(&self, pidx: usize, vidx: usize, day: Day) -> i64 {
        let base = self.products[pidx].variants[vidx].price_cents;
        let f = self.price_factor(pidx, day);
        if f == 1.0 { base } else { round_price(base as f64 * f, &self.m.currency) }
    }

    /// Extra traffic a running promotion attracts (ads, email, word of mouth).
    pub fn promo_boost(&self, day: Day) -> f64 {
        let noon = time::ts_of_day(day) + 12 * MICROS_PER_HOUR;
        self.pricing
            .promos
            .iter()
            .filter(|o| o.start <= noon && noon < o.end)
            .map(|o| if o.products.is_none() { 1.0 + 2.5 * o.pct } else { 1.0 + 0.8 * o.pct })
            .fold(1.0, f64::max)
    }

    /// Best automatic promotion for a product at a time (fraction off).
    pub fn promo_for(&self, pidx: usize, at: Ts) -> Option<&Offer> {
        self.pricing
            .promos
            .iter()
            .filter(|o| o.start <= at && at < o.end && o.products.as_ref().is_none_or(|ps| ps.contains(&pidx)))
            .max_by(|a, b| a.pct.partial_cmp(&b.pct).unwrap())
    }

    pub(super) fn setup_pricing(&mut self, day: Day, ts: Ts, rng: &mut Rng, refs: &crate::sim::reference::Refs) {
        let tenant = self.tenant_rid();
        let mut r = Row::new(price_lists::NCOLS);
        r.set(price_lists::TENANT_ID, tenant)
            .set(price_lists::NAME, if self.b2b { "List prices" } else { "Retail" })
            .set(price_lists::CODE, "default")
            .set(price_lists::CURRENCY_CODE, self.m.currency.clone())
            .set(price_lists::IS_DEFAULT, true)
            .set(price_lists::IS_ACTIVE, true)
            .set(price_lists::STARTS_AT, V::Ts(ts))
            .set(price_lists::CREATED_AT, V::Ts(ts))
            .set(price_lists::UPDATED_AT, V::Ts(ts));
        self.pricing.price_list = Some(self.chunk.emit(price_lists::T, day, r));
        self.setup_tax_tables(day, ts, refs);
        // welcome offer for first orders
        let d = self.discount(day, ts, "Welcome 10% off", 10.0);
        let c = self.coupon(day, ts, d, "WELCOME10", None, Some(1), Some(ts), None, rng);
        self.pricing.welcome = Some(Offer { discount: d, coupon: Some((c, "WELCOME10".into())), pct: 0.10, products: None, start: ts, end: time::end_ts() + 1, name: "Welcome 10% off".into(), promotion: None });
        if self.b2b {
            for (min, pct) in [(10, 0.05), (50, 0.10), (200, 0.15)] {
                let d = self.discount(day, ts, &format!("Volume tier {}+", min), pct * 100.0);
                let mut r = Row::new(price_rules::NCOLS);
                r.set(price_rules::TENANT_ID, tenant)
                    .set(price_rules::DISCOUNT_ID, d)
                    .set(price_rules::NAME, format!("Volume discount {min}+ units"))
                    .set(price_rules::PRIORITY, V::Int(min))
                    .set(price_rules::IS_ACTIVE, true)
                    .set(price_rules::STACKABLE, false)
                    .set(price_rules::STARTS_AT, V::Ts(ts))
                    .set(price_rules::CREATED_AT, V::Ts(ts))
                    .set(price_rules::UPDATED_AT, V::Ts(ts));
                let rule = self.chunk.emit(price_rules::T, day, r);
                let mut r = Row::new(price_rule_conditions::NCOLS);
                r.set(price_rule_conditions::TENANT_ID, tenant)
                    .set(price_rule_conditions::PRICE_RULE_ID, rule)
                    .set(price_rule_conditions::ATTRIBUTE, "line.quantity")
                    .set(price_rule_conditions::OPERATOR, "gte")
                    .set(price_rule_conditions::CONDITION_VALUE, V::Json(format!("{min}")))
                    .set(price_rule_conditions::CREATED_AT, V::Ts(ts))
                    .set(price_rule_conditions::UPDATED_AT, V::Ts(ts));
                self.chunk.emit(price_rule_conditions::T, day, r);
                self.pricing.tiers.push((min, pct, d));
            }
        }
    }

    fn setup_tax_tables(&mut self, day: Day, ts: Ts, refs: &crate::sim::reference::Refs) {
        let w = world();
        let tenant = self.tenant_rid();
        let mut r = Row::new(tax_categories::NCOLS);
        r.set(tax_categories::TENANT_ID, tenant).set(tax_categories::NAME, "Standard rate").set(tax_categories::CODE, "standard").set(tax_categories::CREATED_AT, V::Ts(ts)).set(tax_categories::UPDATED_AT, V::Ts(ts));
        let cat = self.chunk.emit(tax_categories::T, day, r);
        let mut dests = vec![self.country];
        dests.extend(self.cross_border_countries.iter().copied());
        dests.dedup();
        for (k, ci) in dests.iter().enumerate() {
            let c = &w.geo.countries[*ci];
            let mut r = Row::new(tax_zones::NCOLS);
            r.set(tax_zones::TENANT_ID, tenant).set(tax_zones::NAME, c.name.clone()).set(tax_zones::COUNTRY_ID, refs.country[*ci]).set(tax_zones::CREATED_AT, V::Ts(ts)).set(tax_zones::UPDATED_AT, V::Ts(ts));
            let zone = self.chunk.emit(tax_zones::T, day, r);
            // the rates this merchant charges into that country, with their history
            let home = &self.m.country;
            let eu = |x: &str| crate::sim::shopper::is_eu(x);
            let charged: Vec<(String, f64)> = if home == "US" {
                vec![("1900-01-01".into(), 0.0)]
            } else if crate::sim::stories::has(self.m, "S17") && c.iso2 == "CH" {
                let de = w.vat_rate(w.country("DE"), "2024-08-01");
                vec![("1900-01-01".into(), 0.0), ("2024-07-01".into(), de), ("2024-10-01".into(), 0.0)]
            } else if crate::sim::stories::has(self.m, "S17") && c.iso2 == "AT" {
                // S17: the tax-zone migration mapped Austria to the German rate for Q3 2024
                let de = w.vat_rate(w.country("DE"), "2024-08-01");
                let at = w.vat_rate(c, "2024-08-01");
                vec![("1900-01-01".into(), at), ("2024-07-01".into(), de), ("2024-10-01".into(), at)]
            } else if (eu(home) && eu(&c.iso2)) || (k == 0) {
                c.vat.iter().map(|v| (v.from.clone(), v.rate)).collect()
            } else {
                vec![("1900-01-01".into(), 0.0)]
            };
            for (i, (from, rate)) in charged.iter().enumerate() {
                let valid_from = time::ts_of_date(time::parse_date(from)).max(ts.min(time::ts_of_day(0)));
                let valid_to = charged.get(i + 1).map(|(f, _)| time::ts_of_date(time::parse_date(f)));
                if valid_to.is_some_and(|t| t <= valid_from) {
                    continue;
                }
                let mut r = Row::new(tax_rates::NCOLS);
                r.set(tax_rates::TENANT_ID, tenant)
                    .set(tax_rates::TAX_CATEGORY_ID, cat)
                    .set(tax_rates::TAX_ZONE_ID, zone)
                    .set(tax_rates::NAME, if *rate == 0.0 { format!("{} - zero-rated", c.name) } else { format!("{} VAT {}%", c.name, rate) })
                    .set(tax_rates::RATE, V::Dec((rate * 100.0).round() as i128, 4))
                    .set(tax_rates::IS_COMPOUND, false)
                    .set(tax_rates::IS_INCLUSIVE, home != "US")
                    .set(tax_rates::VALID_FROM, V::Ts(valid_from))
                    .set(tax_rates::VALID_TO, V::opt_ts(valid_to))
                    .set(tax_rates::CREATED_AT, V::Ts(ts))
                    .set(tax_rates::UPDATED_AT, V::Ts(ts));
                self.chunk.emit(tax_rates::T, day, r);
            }
        }
    }

    pub(super) fn discount(&mut self, day: Day, ts: Ts, name: &str, pct: f64) -> Rid {
        let mut r = Row::new(discounts::NCOLS);
        r.set(discounts::TENANT_ID, self.tenant_rid())
            .set(discounts::NAME, name.to_string())
            .set(discounts::KIND, "percentage")
            .set(discounts::DISCOUNT_VALUE, V::Dec((pct * 100.0).round() as i128, 2))
            .set(discounts::APPLIES_TO_SHIPPING, false)
            .set(discounts::IS_ACTIVE, true)
            .set(discounts::CREATED_AT, V::Ts(ts))
            .set(discounts::UPDATED_AT, V::Ts(ts));
        self.chunk.emit(discounts::T, day, r)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn coupon(&mut self, day: Day, ts: Ts, discount: Rid, code: &str, usage_limit: Option<i64>, per_customer: Option<i64>, starts: Option<Ts>, expires: Option<Ts>, _rng: &mut Rng) -> Rid {
        let end = time::end_ts();
        let status = if expires.is_some_and(|e| e <= end) { "expired" } else { "active" };
        let mut r = Row::new(coupons::NCOLS);
        r.set(coupons::TENANT_ID, self.tenant_rid())
            .set(coupons::DISCOUNT_ID, discount)
            .set(coupons::CODE, code.to_string())
            .set(coupons::STATUS, status)
            .set(coupons::USAGE_LIMIT, usage_limit.map(V::Int).unwrap_or(V::Null))
            .set(coupons::PER_CUSTOMER_LIMIT, per_customer.map(V::Int).unwrap_or(V::Null))
            .set(coupons::TIMES_REDEEMED, V::Int(0))
            .set(coupons::STARTS_AT, V::opt_ts(starts))
            .set(coupons::EXPIRES_AT, V::opt_ts(expires))
            .set(coupons::CREATED_AT, V::Ts(ts))
            .set(coupons::UPDATED_AT, V::Ts(ts));
        let h = self.chunk.emit_deferred(coupons::T, day, r);
        self.pricing.coupon_rows.insert(h.rid, h);
        h.rid
    }

    /// Price schedule of a new product: yearly increases, seasonal clearance, final sale.
    pub(super) fn price_schedule(&self, rng: &mut Rng, launch: Day, archive: Option<Day>, season: i8) -> Vec<(Day, f64)> {
        use chrono::Datelike;
        let n = time::num_days();
        let mut events: Vec<(Day, f64)> = Vec::new();
        let mut f = 1.0;
        // yearly increase on the merchant's repricing day (inflation 2023-2025)
        for year in 2023..=2025 {
            let d = time::day_of(chrono::NaiveDate::from_ymd_opt(year, 2, 1 + (self.idx % 27) as u32).unwrap());
            if d > launch as i64 + 60 && d < n as i64 && rng.chance(0.65) {
                f *= rng.lognormal(libm::log(if year == 2023 { 1.07 } else { 1.04 }), 0.02);
                events.push((d as Day, f));
            }
        }
        // end-of-season clearance (winter items in February, summer items from mid-August)
        if season != 0 {
            for year in 2023..=2025 {
                let start = if season == 1 { chrono::NaiveDate::from_ymd_opt(year, 2, 1) } else { chrono::NaiveDate::from_ymd_opt(year, 8, 15) }.unwrap();
                let d = time::day_of(start);
                if d > launch as i64 + 90 && d + 45 < n as i64 && rng.chance(0.5) {
                    events.push((d as Day, -0.3 - 0.2 * rng.f64())); // negative = markdown fraction
                    events.push(((d + 45) as Day, 0.0)); // back to full price
                }
                let _ = start.month();
            }
        }
        // final sale before the product is archived
        if let Some(a) = archive {
            if a as i64 - 30 > launch as i64 {
                events.push((a - 30, -0.4));
            }
        }
        events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap()));
        // resolve into absolute factors (markdowns relative to the current full price)
        let mut full = 1.0;
        let mut out = Vec::new();
        for (d, x) in events {
            let factor = if x > 0.0 {
                full = x;
                full
            } else if x == 0.0 {
                full
            } else {
                full * (1.0 + x)
            };
            if archive.is_some_and(|a| d >= a) {
                continue;
            }
            if out.last().is_none_or(|(ld, _)| *ld != d) {
                out.push((d, factor));
            } else {
                out.last_mut().unwrap().1 = factor;
            }
        }
        out
    }

    /// `prices` rows for a product's variants, one per price period.
    pub(super) fn emit_prices(&mut self, day: Day, created: Ts, pidx: usize) {
        let tenant = self.tenant_rid();
        let list = self.pricing.price_list.unwrap();
        let steps = self.products[pidx].price_steps.clone();
        let mut periods: Vec<(Ts, f64, f64)> = vec![(created, 1.0, 1.0)]; // (from, factor, full factor)
        let mut full = 1.0;
        for (d, f) in &steps {
            let from = time::ts_of_day(*d) + 6 * MICROS_PER_HOUR;
            if *f >= full * 0.999 || periods.len() == 1 && *f >= 1.0 {
                full = full.max(*f);
            }
            periods.push((from, *f, full.max(*f)));
        }
        let end = time::end_ts();
        let cur = self.m.currency.clone();
        for vi in 0..self.products[pidx].variants.len() {
            let (vrid, base) = (self.products[pidx].variants[vi].rid, self.products[pidx].variants[vi].price_cents);
            for (k, (from, f, full)) in periods.iter().enumerate() {
                if *from > end {
                    break;
                }
                let to = periods.get(k + 1).map(|p| p.0).filter(|t| *t <= end);
                let amount = if *f == 1.0 { base } else { round_price(base as f64 * f, &cur) };
                let compare = if *f < full * 0.98 { V::Money(round_price(base as f64 * full, &cur)) } else { V::Null };
                let mut r = Row::new(prices::NCOLS);
                r.set(prices::TENANT_ID, tenant)
                    .set(prices::PRICE_LIST_ID, list)
                    .set(prices::VARIANT_ID, vrid)
                    .set(prices::CURRENCY_CODE, cur.clone())
                    .set(prices::AMOUNT, V::Money(amount))
                    .set(prices::COMPARE_AT_AMOUNT, compare)
                    .set(prices::MIN_QUANTITY, V::Int(1))
                    .set(prices::VALID_FROM, V::Ts(*from))
                    .set(prices::VALID_TO, V::opt_ts(to))
                    .set(prices::CREATED_AT, V::Ts(*from))
                    .set(prices::UPDATED_AT, V::Ts(to.unwrap_or(*from)));
                let d = (time::day_of_ts(*from).max(day as i64)) as Day;
                self.chunk.emit(prices::T, d, r);
            }
        }
    }

    /// Seasonal promotions, announced a week ahead.
    pub(super) fn promotions_day(&mut self, day: Day, rng: &mut Rng) {
        use chrono::Datelike;
        if self.b2b {
            return;
        }
        let d = time::date_of(day) + chrono::Duration::days(7);
        let bf = calendar::black_friday(d.year());
        let (name, slug, pct, start, len, sitewide): (String, String, f64, chrono::NaiveDate, i64, bool) = if d == bf - chrono::Duration::days(3) {
            let pct = *rng.pick(&[0.15, 0.2, 0.25, 0.3]);
            (format!("Black Friday {}", d.year()), format!("black-friday-{}", d.year()), pct, d, 8, true)
        } else if d.month() == 1 && d.day() == 2 {
            (format!("January Sale {}", d.year()), format!("january-sale-{}", d.year()), *rng.pick(&[0.2, 0.3, 0.4]), d, 28, false)
        } else if crate::sim::stories::s7_sale_start(self.m, d) {
            ("Spring Mega Sale 2025".to_string(), "spring-mega-sale-2025".to_string(), 0.4, d, 21, true)
        } else if d.month() == 7 && d.day() == 1 {
            (format!("Summer Sale {}", d.year()), format!("summer-sale-{}", d.year()), *rng.pick(&[0.15, 0.25, 0.35]), d, 21, false)
        } else {
            return;
        };
        // not every merchant joins every sale (merchant-specific, stable per sale)
        let story = slug.starts_with("spring-mega");
        let joins = story || (crate::rng::hash_str(&format!("{}{}", self.m.slug, slug)) % 100) < if sitewide { 70 } else { 50 };
        if !joins || time::day_of(start) >= time::num_days() as i64 {
            return;
        }
        let ts = time::ts_of_day(day) + 10 * MICROS_PER_HOUR;
        let start_ts = time::ts_of_date(start);
        let end_ts = start_ts + len * MICROS_PER_DAY;
        let disc = self.discount(day, ts, &name, pct * 100.0);
        let code = if story { Some("MEGA40".to_string()) } else if sitewide { Some(format!("BF{}{}", d.year() % 100, (pct * 100.0) as i64)) } else { None };
        let coupon = code.as_ref().map(|c| (self.coupon(day, ts, disc, c, None, None, Some(start_ts), Some(end_ts), rng), c.clone()));
        let mut r = Row::new(promotions::NCOLS);
        r.set(promotions::TENANT_ID, self.tenant_rid())
            .set(promotions::DISCOUNT_ID, disc)
            .set(promotions::COUPON_ID, V::opt_ref(coupon.as_ref().map(|c| c.0)))
            .set(promotions::NAME, name.clone())
            .set(promotions::SLUG, slug)
            .set(promotions::CAMPAIGN_CODE, code.clone())
            .set(promotions::IS_ACTIVE, end_ts > time::end_ts())
            .set(promotions::STARTS_AT, V::Ts(start_ts))
            .set(promotions::ENDS_AT, V::Ts(end_ts))
            .set(promotions::CREATED_AT, V::Ts(ts))
            .set(promotions::UPDATED_AT, V::Ts(ts));
        let promo = self.chunk.emit(promotions::T, day, r);
        let products = if sitewide {
            None
        } else {
            // sales cover seasonal and older stock
            let picks: Vec<usize> = self.live.iter().copied().filter(|&pi| self.products[pi].season != 0 || rng.chance(0.25)).collect();
            for &pi in &picks {
                let mut r = Row::new(promotion_products::NCOLS);
                r.set(promotion_products::TENANT_ID, self.tenant_rid()).set(promotion_products::PROMOTION_ID, promo).set(promotion_products::PRODUCT_ID, self.products[pi].rid).set(promotion_products::CREATED_AT, V::Ts(ts));
                self.chunk.emit(promotion_products::T, day, r);
            }
            Some(picks)
        };
        self.pricing.promos.push(Offer { discount: disc, coupon, pct, products, start: start_ts, end: end_ts, name, promotion: Some(promo) });
    }

    /// Coupon redemption counts into the (deferred) coupon rows.
    pub fn count_redemptions(&mut self, used: &[Rid]) {
        for c in used {
            *self.pricing.redemptions.entry(*c).or_default() += 1;
        }
        // written at the end (see finish_pricing)
    }

    pub fn finish_pricing(&mut self) {
        let counts: Vec<(DHandle, i64)> = self.pricing.redemptions.iter().filter_map(|(c, n)| self.pricing.coupon_rows.get(c).map(|h| (*h, *n))).collect();
        for (h, n) in counts {
            self.chunk.deferred_mut(h).set(coupons::TIMES_REDEEMED, V::Int(n));
        }
    }
}
