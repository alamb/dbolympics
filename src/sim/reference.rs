//! Global reference data: countries, regions, cities, currencies, locales, time zones, FX.
//! Written on day 0 by the global chunk; ids follow file order, so they are stable.

use std::collections::BTreeMap;

use crate::engine::Chunk;
use crate::rng::Rng;
use crate::tables::*;
use crate::time;
use crate::value::{Rid, Row, V};
use crate::world::world;

/// Ids of reference rows, for the other modules.
pub struct Refs {
    pub country: Vec<Rid>,                          // by index in world().geo.countries
    pub regions: Vec<Vec<Rid>>,                     // country -> regions
    #[allow(dead_code)] // addresses store the city name, not a city id
    pub cities: Vec<Vec<Vec<Rid>>>,                 // country -> region -> cities
    #[allow(dead_code)] // no table references time zones by id yet
    pub timezone: BTreeMap<String, Rid>,
    /// the platform's permission catalog
    pub permissions: Vec<(&'static str, Rid)>,
}

/// (code, category, description) of the platform permissions.
pub const PERMISSIONS: &[(&str, &str, &str)] = &[
    ("orders.read", "orders", "View orders"),
    ("orders.write", "orders", "Edit and cancel orders"),
    ("orders.refund", "orders", "Issue refunds"),
    ("products.read", "catalog", "View products"),
    ("products.write", "catalog", "Create and edit products and prices"),
    ("inventory.read", "inventory", "View stock levels"),
    ("inventory.write", "inventory", "Adjust stock, receive purchase orders"),
    ("customers.read", "customers", "View customer profiles"),
    ("customers.write", "customers", "Edit customer profiles"),
    ("customers.export", "customers", "Export customer data"),
    ("marketing.read", "marketing", "View campaigns and segments"),
    ("marketing.write", "marketing", "Create campaigns, discounts and segments"),
    ("reports.read", "analytics", "View reports and dashboards"),
    ("support.tickets", "support", "Work on support tickets"),
    ("support.admin", "support", "Manage macros, SLAs and the help centre"),
    ("crm.read", "crm", "View accounts and deals"),
    ("crm.write", "crm", "Manage accounts, deals and quotes"),
    ("billing.read", "billing", "View invoices, payouts and subscriptions"),
    ("billing.write", "billing", "Manage plans, invoices and credit notes"),
    ("settings.read", "settings", "View store settings"),
    ("settings.write", "settings", "Change store settings"),
    ("users.manage", "settings", "Invite staff and assign roles"),
    ("api_keys.manage", "settings", "Create and revoke API keys"),
    ("integrations.manage", "settings", "Connect and configure integrations"),
    ("audit.read", "compliance", "View the audit log"),
    ("gdpr.manage", "compliance", "Handle data subject requests"),
];

const TIMEZONES: &[(&str, i32)] = &[
    ("UTC", 0), ("Europe/London", 0), ("Europe/Dublin", 0), ("Europe/Lisbon", 0), ("Europe/Amsterdam", 60),
    ("Europe/Brussels", 60), ("Europe/Luxembourg", 60), ("Europe/Paris", 60), ("Europe/Berlin", 60),
    ("Europe/Vienna", 60), ("Europe/Zurich", 60), ("Europe/Madrid", 60), ("Europe/Rome", 60),
    ("Europe/Stockholm", 60), ("Europe/Copenhagen", 60), ("Europe/Oslo", 60), ("Europe/Warsaw", 60),
    ("Europe/Prague", 60), ("Europe/Bratislava", 60), ("Europe/Budapest", 60), ("Europe/Helsinki", 120),
    ("Europe/Tallinn", 120), ("Europe/Athens", 120), ("Europe/Bucharest", 120), ("America/New_York", -300),
    ("America/Chicago", -360), ("America/Denver", -420), ("America/Phoenix", -420), ("America/Los_Angeles", -480),
    ("America/Toronto", -300), ("America/Vancouver", -480), ("America/Mexico_City", -360), ("America/Sao_Paulo", -180),
    ("Australia/Sydney", 600), ("Pacific/Auckland", 720), ("Asia/Tokyo", 540), ("Asia/Singapore", 480),
    ("Asia/Hong_Kong", 480), ("Asia/Dubai", 240), ("Asia/Kolkata", 330), ("Africa/Johannesburg", 120),
];

pub fn emit(c: &mut Chunk, seed: u64) -> Refs {
    let w = world();
    let mut rng = Rng::new(seed, &[0xFE, 1]);

    for cur in &w.geo.currencies {
        let mut r = Row::new(currencies::NCOLS);
        r.set(currencies::CODE, cur.code.clone())
            .set(currencies::NAME, cur.name.clone())
            .set(currencies::SYMBOL, cur.symbol.clone())
            .set(currencies::MINOR_UNIT, cur.minor_unit);
        c.emit(currencies::T, 0, r);
    }
    for l in &w.geo.locales {
        let mut r = Row::new(locales::NCOLS);
        r.set(locales::CODE, l.code.clone()).set(locales::NAME, l.name.clone()).set(locales::NATIVE_NAME, l.native_name.clone());
        c.emit(locales::T, 0, r);
    }
    let mut timezone = BTreeMap::new();
    for (name, off) in TIMEZONES {
        let mut r = Row::new(timezones::NCOLS);
        r.set(timezones::NAME, *name).set(timezones::UTC_OFFSET_MINUTES, *off);
        timezone.insert(name.to_string(), c.emit(timezones::T, 0, r));
    }

    let mut country = Vec::new();
    for k in &w.geo.countries {
        let mut r = Row::new(countries::NCOLS);
        r.set(countries::ISO2, k.iso2.clone())
            .set(countries::ISO3, k.iso3.clone())
            .set(countries::NAME, k.name.clone())
            .set(countries::PHONE_CODE, k.phone_code.clone())
            .set(countries::CONTINENT, k.continent.clone());
        country.push(c.emit(countries::T, 0, r));
    }
    let mut regions_out = Vec::new();
    let mut cities_out = Vec::new();
    for (ci, k) in w.geo.countries.iter().enumerate() {
        let mut regs = Vec::new();
        let mut cits = Vec::new();
        for reg in &k.regions {
            let mut r = Row::new(regions::NCOLS);
            r.set(regions::COUNTRY_ID, country[ci]).set(regions::CODE, reg.code.clone()).set(regions::NAME, reg.name.clone());
            let rid = c.emit(regions::T, 0, r);
            let mut cs = Vec::new();
            for city in &reg.cities {
                let mut r = Row::new(cities::NCOLS);
                r.set(cities::REGION_ID, rid)
                    .set(cities::COUNTRY_ID, country[ci])
                    .set(cities::NAME, city.name.clone())
                    .set(cities::LATITUDE, V::Dec((city.lat * 1e6).round() as i128, 6))
                    .set(cities::LONGITUDE, V::Dec((city.lon * 1e6).round() as i128, 6));
                cs.push(c.emit(cities::T, 0, r));
            }
            regs.push(rid);
            cits.push(cs);
        }
        regions_out.push(regs);
        cities_out.push(cits);
    }

    // Daily EUR reference rates: a random walk drifting between the start and end levels in
    // world/geo.json (synthetic, see docs/DATA-SOURCES.md). Rates move on business days; the
    // warehouse forward-fills weekends with the last business day's rate, so every day has one.
    let n = time::num_days();
    for cur in &w.geo.currencies {
        let Some(span) = &cur.eur_rate else { continue };
        let mut walk = 0.0f64;
        let vol = if cur.code == "DKK" { 0.0004 } else { 0.004 };
        let mut rate = span.start;
        for day in 0..n {
            let date = time::date_of(day);
            use chrono::Datelike;
            if date.weekday().number_from_monday() <= 5 {
                walk = 0.97 * walk + vol * rng.normal();
                let f = day as f64 / (n - 1) as f64;
                let level = span.start * (1.0 - f) + span.end * f;
                rate = level * libm::exp(walk);
            }
            let mut r = Row::new(exchange_rates::NCOLS);
            r.set(exchange_rates::BASE_CODE, "EUR")
                .set(exchange_rates::QUOTE_CODE, cur.code.clone())
                .set(exchange_rates::RATE, V::Dec((rate * 1e6).round() as i128, 6))
                .set(exchange_rates::AS_OF, V::Date(time::epoch_days(date)));
            c.emit(exchange_rates::T, 0, r);
        }
    }
    // calendar dimension: every day of the history plus the planning year 2026
    {
        use chrono::Datelike;
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
        let ymd = |y, m, d| chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let mut d = ymd(2023, 1, 1);
        while d <= ymd(2026, 12, 31) {
            let dow = d.weekday().number_from_monday();
            let q = (d.month() - 1) / 3 + 1;
            let mut r = Row::new(calendar_days::NCOLS);
            r.set(calendar_days::CALENDAR_DATE, V::Date(time::epoch_days(d)))
                .set(calendar_days::YEAR_NUM, d.year())
                .set(calendar_days::QUARTER_NUM, q)
                .set(calendar_days::MONTH_NUM, d.month())
                .set(calendar_days::MONTH_NAME, MONTHS[d.month0() as usize])
                .set(calendar_days::MONTH_LABEL, format!("{}-{:02}", d.year(), d.month()))
                .set(calendar_days::DAY_OF_MONTH, d.day())
                .set(calendar_days::DAY_OF_YEAR, d.ordinal())
                .set(calendar_days::ISO_YEAR, d.iso_week().year())
                .set(calendar_days::ISO_WEEK, d.iso_week().week())
                .set(calendar_days::ISO_DAY_OF_WEEK, dow)
                .set(calendar_days::DAY_NAME, DAYS[dow as usize - 1])
                .set(calendar_days::IS_WEEKEND, dow >= 6)
                .set(calendar_days::WEEK_START, V::Date(time::epoch_days(d - chrono::Duration::days(dow as i64 - 1))))
                .set(calendar_days::MONTH_START, V::Date(time::epoch_days(ymd(d.year(), d.month(), 1))))
                .set(calendar_days::QUARTER_START, V::Date(time::epoch_days(ymd(d.year(), 3 * q - 2, 1))));
            c.emit(calendar_days::T, 0, r);
            d = d.succ_opt().unwrap();
        }
    }
    // postal zones: the leading part of each city's postal code format
    for (ci, k) in w.geo.countries.iter().enumerate() {
        let mut seen = std::collections::BTreeSet::new();
        for reg in &k.regions {
            for city in &reg.cities {
                for pat in city.postal.split('|') {
                    let code: String = pat.chars().take_while(|ch| *ch != '#' && *ch != '@' && *ch != ' ').collect();
                    if code.is_empty() || !seen.insert(code.clone()) {
                        continue;
                    }
                    let mut r = Row::new(postal_zones::NCOLS);
                    r.set(postal_zones::COUNTRY_ID, country[ci]).set(postal_zones::CODE, code).set(postal_zones::NAME, format!("{} ({})", city.name, reg.name));
                    c.emit(postal_zones::T, 0, r);
                }
            }
        }
    }
    let mut permissions = Vec::new();
    for (code, cat, desc) in PERMISSIONS {
        let mut r = Row::new(permissions::NCOLS);
        r.set(permissions::CODE, *code).set(permissions::DESCRIPTION, *desc).set(permissions::CATEGORY, *cat);
        permissions.push((*code, c.emit(permissions::T, 0, r)));
    }
    Refs { country, regions: regions_out, cities: cities_out, timezone, permissions }
}
