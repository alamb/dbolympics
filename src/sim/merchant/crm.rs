//! Merchant side of the B2B CRM: pipelines and stages, contact lists, sales reps, and leads
//! from trade fairs that never become customers. Accounts, contacts, opportunities and deals
//! of actual buyers live in the shopper chunks (see shopper/b2b.rs). Also the merchant's own
//! tax registrations (all merchants).

use super::MerchantSim;
use crate::rng::Rng;
use crate::sim::people;
use crate::tables::*;
use crate::time::{self, Day, MICROS_PER_DAY, MICROS_PER_HOUR, Ts};
use crate::value::{Rid, Row, V};
use crate::world::world;

/// (name, win probability, won, lost)
const NEW_BUSINESS: [(&str, f64, bool, bool); 6] = [("Prospecting", 0.1, false, false), ("Qualification", 0.25, false, false), ("Proposal", 0.5, false, false), ("Negotiation", 0.75, false, false), ("Closed Won", 1.0, true, false), ("Closed Lost", 0.0, false, true)];
const RENEWALS: [(&str, f64, bool, bool); 4] = [("Renewal due", 0.6, false, false), ("Proposal sent", 0.75, false, false), ("Renewed", 1.0, true, false), ("Churned", 0.0, false, true)];

#[derive(Default)]
pub struct Crm {
    /// (pipeline, stages in order)
    pub new_business: Option<(Rid, Vec<Rid>)>,
    pub renewals: Option<(Rid, Vec<Rid>)>,
    pub reps: Vec<Rid>,
    /// key accounts, B2B newsletter
    pub lists: Vec<Rid>,
    /// S20: the win-back contact list
    pub winback: Option<Rid>,
}

/// A VAT / tax id in the country's format (random digits, valid-looking only).
pub fn tax_id(rng: &mut Rng, iso2: &str) -> String {
    match iso2 {
        "DE" => format!("DE{}", rng.digits(9)),
        "GB" => format!("GB{}", rng.digits(9)),
        "NL" => format!("NL{}B01", rng.digits(9)),
        "FR" => format!("FR{}{}", rng.digits(2), rng.digits(9)),
        "BE" => format!("BE0{}", rng.digits(9)),
        "IE" => format!("IE{}{}", rng.digits(7), (b'A' + rng.below(26) as u8) as char),
        "AT" => format!("ATU{}", rng.digits(8)),
        "NO" => format!("NO{}MVA", rng.digits(9)),
        "SE" => format!("SE{}01", rng.digits(10)),
        "DK" => format!("DK{}", rng.digits(8)),
        "FI" => format!("FI{}", rng.digits(8)),
        "CH" => format!("CHE-{}.{}.{} MWST", rng.digits(3), rng.digits(3), rng.digits(3)),
        "US" => format!("{}-{}", rng.digits(2), rng.digits(7)),
        x => format!("{x}{}", rng.digits(9)),
    }
}

impl MerchantSim {
    fn pipeline(&mut self, day: Day, ts: Ts, name: &str, default: bool, stages: &[(&'static str, f64, bool, bool)]) -> (Rid, Vec<Rid>) {
        let tenant = self.tenant_rid();
        let mut r = Row::new(pipelines::NCOLS);
        r.set(pipelines::TENANT_ID, tenant).set(pipelines::NAME, name.to_string()).set(pipelines::IS_DEFAULT, default).set(pipelines::IS_ACTIVE, true).set(pipelines::CREATED_AT, V::Ts(ts)).set(pipelines::UPDATED_AT, V::Ts(ts));
        let p = self.chunk.emit(pipelines::T, day, r);
        let mut v = Vec::new();
        for (i, (n, prob, won, lost)) in stages.iter().enumerate() {
            let mut r = Row::new(opportunity_stages::NCOLS);
            r.set(opportunity_stages::TENANT_ID, tenant)
                .set(opportunity_stages::PIPELINE_ID, p)
                .set(opportunity_stages::NAME, *n)
                .set(opportunity_stages::SORT_POSITION, i)
                .set(opportunity_stages::WIN_PROBABILITY, V::Dec((prob * 10000.0) as i128, 4))
                .set(opportunity_stages::IS_WON, *won)
                .set(opportunity_stages::IS_LOST, *lost)
                .set(opportunity_stages::CREATED_AT, V::Ts(ts))
                .set(opportunity_stages::UPDATED_AT, V::Ts(ts));
            v.push(self.chunk.emit(opportunity_stages::T, day, r));
        }
        (p, v)
    }

    pub(super) fn setup_crm(&mut self, day: Day, ts: Ts, rng: &mut Rng) {
        if !self.b2b {
            return;
        }
        let tenant = self.tenant_rid();
        self.crm.new_business = Some(self.pipeline(day, ts, "New business", true, &NEW_BUSINESS));
        self.crm.renewals = Some(self.pipeline(day, ts, "Renewals", false, &RENEWALS));
        self.crm.reps = self.staff.iter().filter(|s| s.role == "sales").map(|s| s.user.rid).collect();
        if self.crm.reps.is_empty() {
            self.crm.reps = self.staff.iter().filter(|s| s.role == "admin" || s.role == "owner").map(|s| s.user.rid).collect();
        }
        let owner = self.crm.reps[0];
        for (name, desc) in [("Key accounts", "Largest customers, quarterly business reviews"), ("B2B newsletter", "Contacts who receive the monthly trade newsletter")] {
            let mut r = Row::new(contact_lists::NCOLS);
            r.set(contact_lists::TENANT_ID, tenant).set(contact_lists::OWNER_USER_ID, owner).set(contact_lists::NAME, name).set(contact_lists::DESCRIPTION, if rng.chance(0.7) { V::Static(desc) } else { V::Null }).set(contact_lists::CREATED_AT, V::Ts(ts)).set(contact_lists::UPDATED_AT, V::Ts(ts));
            self.crm.lists.push(self.chunk.emit(contact_lists::T, day, r));
        }
    }

    /// Yearly trade fairs bring leads; most never order.
    pub(super) fn crm_day(&mut self, day: Day, rng: &mut Rng, sf: f64) {
        use chrono::Datelike;
        if !self.b2b || self.crm.reps.is_empty() {
            return;
        }
        if crate::sim::stories::s20_price_list(self.m, day) {
            self.s20_price_list(day, rng);
        }
        if crate::sim::stories::has(self.m, "S20") && self.crm.winback.is_none() && day == time::day_of(chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()) as Day {
            let at = time::ts_of_day(day) + 9 * MICROS_PER_HOUR;
            let mut r = Row::new(contact_lists::NCOLS);
            r.set(contact_lists::TENANT_ID, self.tenant_rid()).set(contact_lists::OWNER_USER_ID, self.crm.reps[0]).set(contact_lists::NAME, "Win-back 2025").set(contact_lists::DESCRIPTION, V::Static("Former accounts to call about the new volume prices")).set(contact_lists::CREATED_AT, V::Ts(at)).set(contact_lists::UPDATED_AT, V::Ts(at));
            self.crm.winback = Some(self.chunk.emit(contact_lists::T, day, r));
        }
        let d = time::date_of(day);
        if !(d.month() == 3 + (self.idx % 6) as u32 && d.day() == 10) {
            return;
        }
        let tenant = self.tenant_rid();
        let end = time::end_ts();
        let w = world();
        let c = w.country(&self.m.country);
        let n = rng.poisson(25.0 * sf.sqrt() * libm::sqrt(self.share / 0.02));
        let name = format!("Trade fair leads {}", d.year());
        let at0 = self.tz.local_to_utc(d, 17 * MICROS_PER_HOUR);
        let mut r = Row::new(contact_lists::NCOLS);
        r.set(contact_lists::TENANT_ID, tenant).set(contact_lists::OWNER_USER_ID, self.crm.reps[0]).set(contact_lists::NAME, name).set(contact_lists::CREATED_AT, V::Ts(at0)).set(contact_lists::UPDATED_AT, V::Ts(at0));
        self.chunk.emit(contact_lists::T, day, r);
        for _ in 0..n {
            let p = people::person(rng, c);
            let company = format!("{} {}", p.last, rng.pick(&["Group", "Services", "Bau", "Technik", "Facilities", "Supplies", "& Co"]));
            let created = at0 + rng.int(0, 3) * MICROS_PER_DAY + rng.int(0, 8 * 3600) * 1_000_000;
            let status = *rng.pick(&["new", "contacted", "contacted", "qualified", "unqualified", "unqualified"]);
            let owner = self.crm.reps[rng.below(self.crm.reps.len())];
            let mut r = Row::new(leads::NCOLS);
            r.set(leads::TENANT_ID, tenant)
                .set(leads::OWNER_USER_ID, owner)
                .set(leads::FIRST_NAME, p.first.clone())
                .set(leads::LAST_NAME, p.last.clone())
                .set(leads::EMAIL, if rng.chance(0.9) { V::Str(format!("{}@{}.{}", people::ascii_fold(&p.first).to_lowercase().replace(' ', ""), people::slugify(&company).replace('-', ""), if self.m.country == "US" { "com" } else { "eu" })) } else { V::Null })
                .set(leads::PHONE, if rng.chance(0.5) { V::Str(people::phone(rng, &self.m.country)) } else { V::Null })
                .set(leads::COMPANY, company)
                .set(leads::JOB_TITLE, if rng.chance(0.7) { V::Static(*rng.pick(&["Purchasing Manager", "Facility Manager", "Office Manager", "Managing Director", "Buyer"])) } else { V::Null })
                .set(leads::STATUS, status)
                .set(leads::SOURCE, "trade_fair")
                .set(leads::SCORE, V::Int(match status { "qualified" => rng.int(60, 95), "unqualified" => rng.int(0, 25), _ => rng.int(10, 60) }))
                .set(leads::CREATED_AT, V::Ts(created))
                .set(leads::UPDATED_AT, V::Ts((created + rng.int(0, 60) * MICROS_PER_DAY).min(end)));
            self.chunk.emit(leads::T, day, r);
        }
    }

    /// S20: the answer to the price war: a volume price list with deeper tiers.
    fn s20_price_list(&mut self, day: Day, rng: &mut Rng) {
        let tenant = self.tenant_rid();
        let at = time::ts_of_day(day) + 7 * MICROS_PER_HOUR;
        let mut r = Row::new(price_lists::NCOLS);
        r.set(price_lists::TENANT_ID, tenant).set(price_lists::NAME, "Volume pricing 2025").set(price_lists::CODE, "volume-2025").set(price_lists::CURRENCY_CODE, self.m.currency.clone()).set(price_lists::IS_DEFAULT, false).set(price_lists::IS_ACTIVE, true).set(price_lists::STARTS_AT, V::Ts(at)).set(price_lists::CREATED_AT, V::Ts(at - 5 * MICROS_PER_DAY)).set(price_lists::UPDATED_AT, V::Ts(at));
        let list = self.chunk.emit(price_lists::T, day, r);
        let mut live: Vec<(f64, usize)> = self.live.iter().map(|&i| (self.products[i].appeal, i)).collect();
        live.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        for (_, pi) in live.into_iter().take(50) {
            for vi in 0..self.products[pi].variants.len().min(3) {
                let now = self.price_now(pi, vi, day);
                let mut r = Row::new(prices::NCOLS);
                r.set(prices::TENANT_ID, tenant)
                    .set(prices::PRICE_LIST_ID, list)
                    .set(prices::VARIANT_ID, self.products[pi].variants[vi].rid)
                    .set(prices::CURRENCY_CODE, self.m.currency.clone())
                    .set(prices::AMOUNT, V::Money(super::pricing::round_price(now as f64 * 0.92, &self.m.currency)))
                    .set(prices::COMPARE_AT_AMOUNT, V::Money(now))
                    .set(prices::MIN_QUANTITY, V::Int(*rng.pick(&[5, 10, 25])))
                    .set(prices::VALID_FROM, V::Ts(at))
                    .set(prices::CREATED_AT, V::Ts(at))
                    .set(prices::UPDATED_AT, V::Ts(at));
                self.chunk.emit(prices::T, day, r);
            }
        }
        let mut tiers = Vec::new();
        for (min, pct) in [(5, 0.08), (25, 0.14), (100, 0.2)] {
            let d = self.discount(day, at, &format!("Volume 2025 tier {min}+"), pct * 100.0);
            let mut r = Row::new(price_rules::NCOLS);
            r.set(price_rules::TENANT_ID, tenant).set(price_rules::DISCOUNT_ID, d).set(price_rules::NAME, format!("Volume 2025: {min}+ units")).set(price_rules::PRIORITY, V::Int(1000 + min)).set(price_rules::IS_ACTIVE, true).set(price_rules::STACKABLE, false).set(price_rules::STARTS_AT, V::Ts(at)).set(price_rules::CREATED_AT, V::Ts(at)).set(price_rules::UPDATED_AT, V::Ts(at));
            let rule = self.chunk.emit(price_rules::T, day, r);
            let mut r = Row::new(price_rule_conditions::NCOLS);
            r.set(price_rule_conditions::TENANT_ID, tenant).set(price_rule_conditions::PRICE_RULE_ID, rule).set(price_rule_conditions::ATTRIBUTE, "line.quantity").set(price_rule_conditions::OPERATOR, "gte").set(price_rule_conditions::CONDITION_VALUE, V::Json(format!("{min}"))).set(price_rule_conditions::CREATED_AT, V::Ts(at)).set(price_rule_conditions::UPDATED_AT, V::Ts(at));
            self.chunk.emit(price_rule_conditions::T, day, r);
            tiers.push((min, pct, d));
        }
        // the new tiers replace the old ones at checkout
        self.pricing.tiers = tiers;
    }

    /// The merchant's own tax registrations (VAT, the EU One-Stop-Shop, US sales tax).
    pub(super) fn tax_registrations(&mut self, day: Day, ts: Ts, rng: &mut Rng) {
        let tenant = self.tenant_rid();
        let iso = self.m.country.clone();
        let joined = time::parse_date(&self.m.joined_on);
        let mut regs: Vec<(String, Option<String>, &str, String, chrono::NaiveDate)> = Vec::new();
        if iso == "US" {
            let st = format!("US-{}", ["TX", "CA", "NY", "IL", "WA"][self.idx % 5]);
            regs.push((iso.clone(), Some(st), "sales_tax", rng.digits(11), joined));
        } else {
            regs.push((iso.clone(), None, "VAT", tax_id(rng, &iso), joined - chrono::Duration::days(rng.int(200, 2000))));
            if crate::sim::shopper::is_eu(&iso) && !self.cross_border_countries.is_empty() {
                // the EU One-Stop-Shop started on 2021-07-01
                regs.push((iso.clone(), Some("EU".into()), "OSS", format!("EU{}", rng.digits(9)), joined.max(chrono::NaiveDate::from_ymd_opt(2021, 7, 1).unwrap())));
            }
            if crate::sim::stories::has(self.m, "S17") {
                // registered for VAT in Austria after the Q3 2024 audit
                regs.push(("AT".into(), None, "VAT", tax_id(rng, "AT"), chrono::NaiveDate::from_ymd_opt(2024, 10, 15).unwrap()));
            }
            if iso == "GB" && self.cross_border_countries.iter().any(|c| world().geo.countries[*c].iso2 == "IE") {
                regs.push(("IE".into(), None, "VAT", tax_id(rng, "IE"), chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap().max(joined)));
            }
        }
        for (country, jur, kind, number, from) in regs {
            let mut r = Row::new(tax_registrations::NCOLS);
            r.set(tax_registrations::TENANT_ID, tenant)
                .set(tax_registrations::COUNTRY_CODE, country)
                .set(tax_registrations::JURISDICTION, jur)
                .set(tax_registrations::TAX_TYPE, kind)
                .set(tax_registrations::REGISTRATION_NUMBER, number)
                .set(tax_registrations::IS_ACTIVE, true)
                .set(tax_registrations::VALID_FROM, V::Date(time::epoch_days(from)))
                .set(tax_registrations::CREATED_AT, V::Ts(ts))
                .set(tax_registrations::UPDATED_AT, V::Ts(ts));
            self.chunk.emit(tax_registrations::T, day, r);
        }
    }
}
