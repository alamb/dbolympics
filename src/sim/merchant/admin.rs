//! The merchant's back office: roles and permissions, teams, staff invitations and logins,
//! API keys and service accounts, settings, feature flags, background jobs, integrations,
//! imports, health checks, retention policies, and the audit trail.
//!
//! Audit rows older than 24 months are moved to `audit_log_archive` by the monthly archival
//! job: they keep their id (the gap stays in `audit_log`) and the archive row points to it.

use std::collections::BTreeMap;

use super::MerchantSim;
use crate::engine::Chunk;
use crate::rng::Rng;
use crate::schema::{TableId, table};
use crate::sim::people;
use crate::sim::reference::Refs;
use crate::tables::*;
use crate::time::{self, Day, MICROS_PER_DAY, MICROS_PER_HOUR, MICROS_PER_MIN, MICROS_PER_SEC, Ts};
use crate::value::{Rid, Row, V};

/// One audit trail entry.
pub struct Audit<'a> {
    pub at: Ts,
    pub action: &'static str,
    pub actor: Option<Rid>,
    pub actor_type: &'static str,
    pub entity: Option<(TableId, Rid)>,
    pub summary: String,
    pub metadata: String,
    pub ip: Option<&'a str>,
    pub ua: Option<&'a str>,
}

/// When the monthly archival job (1st of the month, 03:00 UTC) moves a row of time `at`.
fn archived_at(at: Ts) -> Option<Ts> {
    use chrono::Datelike;
    let d = chrono::DateTime::from_timestamp_micros(at).unwrap().date_naive();
    let due = d.checked_add_months(chrono::Months::new(24)).unwrap();
    let first = if due.day() == 1 && at < time::ts_of_date(due) + 3 * MICROS_PER_HOUR { due } else { chrono::NaiveDate::from_ymd_opt(due.year(), due.month(), 1).unwrap().checked_add_months(chrono::Months::new(1)).unwrap() };
    let t = time::ts_of_date(first) + 3 * MICROS_PER_HOUR;
    (t <= time::end_ts()).then_some(t)
}

/// Write an audit entry; returns its id unless it has been archived since.
pub fn audit(c: &mut Chunk, rng: &mut Rng, tenant: Rid, a: Audit) -> Option<Rid> {
    // the audit trail starts with the 2023 migration
    if a.at > time::end_ts() || a.at < time::ts_of_day(0) {
        return None;
    }
    let (schema_, table_, id) = match a.entity {
        Some((t, rid)) => (V::Str(table(t).module.clone()), V::Str(table(t).name.clone()), V::Ref(rid)),
        None => (V::Null, V::Null, V::Null),
    };
    let request = format!("req_{}", rng.alnum(20));
    let mut r = Row::new(audit_log::NCOLS);
    r.set(audit_log::TENANT_ID, tenant)
        .set(audit_log::ACTION, a.action)
        .set(audit_log::ACTOR_USER_ID, V::opt_ref(a.actor))
        .set(audit_log::ACTOR_TYPE, a.actor_type)
        .set(audit_log::ENTITY_SCHEMA, schema_.clone())
        .set(audit_log::ENTITY_TABLE, table_.clone())
        .set(audit_log::ENTITY_ID, id.clone())
        .set(audit_log::SUMMARY, a.summary.clone())
        .set(audit_log::METADATA, V::Json(a.metadata.clone()))
        .set(audit_log::IP_ADDRESS, a.ip.map(String::from))
        .set(audit_log::USER_AGENT, a.ua.map(String::from))
        .set(audit_log::REQUEST_ID, request.clone())
        .set(audit_log::OCCURRED_AT, V::Ts(a.at))
        .set(audit_log::CREATED_AT, V::Ts(a.at + rng.int(1, 900_000)));
    let archived = archived_at(a.at);
    r.skip = archived.is_some();
    let day = time::day_of_ts(a.at).max(c.day as i64) as Day;
    let rid = c.emit(audit_log::T, day, r);
    if let Some(t) = archived {
        let mut r = Row::new(audit_log_archive::NCOLS);
        r.set(audit_log_archive::SOURCE_AUDIT_LOG_ID, rid)
            .set(audit_log_archive::TENANT_ID, tenant)
            .set(audit_log_archive::ACTION, a.action)
            .set(audit_log_archive::ACTOR_USER_ID, V::opt_ref(a.actor))
            .set(audit_log_archive::ACTOR_TYPE, a.actor_type)
            .set(audit_log_archive::ENTITY_SCHEMA, schema_)
            .set(audit_log_archive::ENTITY_TABLE, table_)
            .set(audit_log_archive::ENTITY_ID, id)
            .set(audit_log_archive::SUMMARY, a.summary)
            .set(audit_log_archive::METADATA, V::Json(a.metadata))
            .set(audit_log_archive::IP_ADDRESS, a.ip.map(String::from))
            .set(audit_log_archive::USER_AGENT, a.ua.map(String::from))
            .set(audit_log_archive::REQUEST_ID, request)
            .set(audit_log_archive::OCCURRED_AT, V::Ts(a.at))
            .set(audit_log_archive::ARCHIVED_AT, V::Ts(t))
            .set(audit_log_archive::CREATED_AT, V::Ts(t + rng.int(1, 600) * MICROS_PER_SEC));
        let d = time::day_of_ts(t).max(c.day as i64) as Day;
        c.emit(audit_log_archive::T, d, r);
        return None;
    }
    Some(rid)
}

/// Column-level before/after diff of an audited change.
#[allow(clippy::too_many_arguments)]
pub fn change(c: &mut Chunk, tenant: Rid, audit_row: Option<Rid>, actor: Option<Rid>, entity: (TableId, Rid), cols: &[&str], before: String, after: String, at: Ts) {
    if at > time::end_ts() {
        return;
    }
    let mut r = Row::new(change_history::NCOLS);
    r.set(change_history::TENANT_ID, tenant)
        .set(change_history::AUDIT_LOG_ID, V::opt_ref(audit_row))
        .set(change_history::ACTOR_USER_ID, V::opt_ref(actor))
        .set(change_history::ENTITY_SCHEMA, table(entity.0).module.clone())
        .set(change_history::ENTITY_TABLE, table(entity.0).name.clone())
        .set(change_history::ENTITY_ID, entity.1)
        .set(change_history::CHANGED_COLUMNS, V::Arr(cols.iter().map(|s| s.to_string()).collect()))
        .set(change_history::BEFORE_DATA, V::Json(before))
        .set(change_history::AFTER_DATA, V::Json(after))
        .set(change_history::OCCURRED_AT, V::Ts(at))
        .set(change_history::CREATED_AT, V::Ts(at + 50_000));
    let day = time::day_of_ts(at).max(c.day as i64) as Day;
    c.emit(change_history::T, day, r);
}

#[derive(Clone, Copy)]
enum Cadence {
    /// hour and minute (UTC or local, see `local`)
    Daily(i64, i64),
    /// weekday (0 = Monday), hour
    Weekly(u32, i64),
    /// 1st of the month, hour
    Monthly(i64),
    Hourly,
}

struct Job {
    rid: Rid,
    cadence: Cadence,
    local: bool,
    mean_ms: f64,
    fail: f64,
}

struct Integration {
    rid: Rid,
    every_h: i64,
    direction: &'static str,
    kind: &'static str,
}

struct StaffMeta {
    ip: String,
    ua: &'static str,
    created: Ts,
}

#[derive(Default)]
pub struct Admin {
    /// role name -> role
    pub roles: BTreeMap<&'static str, Rid>,
    staff: Vec<StaffMeta>,
    pub office_ip: String,
    jobs: Vec<Job>,
    integrations: Vec<Integration>,
    /// health-checked components (and their integration)
    health: Vec<(&'static str, Option<Rid>)>,
}

fn role_name(staff_role: &str) -> &'static str {
    match staff_role {
        "owner" => "Owner",
        "admin" => "Administrator",
        "support" => "Support Agent",
        "warehouse" => "Warehouse",
        "marketing" => "Marketing",
        "merchandiser" => "Merchandiser",
        "finance" => "Finance",
        _ => "Sales Rep",
    }
}

fn role_permissions(role: &str) -> &'static [&'static str] {
    match role {
        "Owner" | "Administrator" => &[],
        "Support Agent" => &["orders.read", "orders.refund", "customers.read", "customers.write", "support.tickets", "products.read"],
        "Warehouse" => &["orders.read", "inventory.read", "inventory.write", "products.read"],
        "Marketing" => &["marketing.read", "marketing.write", "reports.read", "customers.read", "products.read"],
        "Merchandiser" => &["products.read", "products.write", "inventory.read", "reports.read"],
        "Finance" => &["orders.read", "billing.read", "billing.write", "reports.read", "customers.export"],
        "Sales Rep" => &["crm.read", "crm.write", "orders.read", "customers.read", "products.read"],
        _ => &["orders.read", "products.read", "reports.read", "customers.read"],
    }
}

/// Last and next run of a cadence around the end of the window.
fn last_next(c: Cadence, tz: &crate::time::Tz, local: bool) -> (Ts, Ts) {
    use chrono::Datelike;
    let end = time::end_ts();
    let last_day = time::num_days() - 1;
    let at = |day: Day, h: i64, m: i64| if local { tz.local_to_utc(time::date_of(day), h * MICROS_PER_HOUR + m * MICROS_PER_MIN) } else { time::ts_of_day(day) + h * MICROS_PER_HOUR + m * MICROS_PER_MIN };
    match c {
        Cadence::Hourly => {
            let l = end - end.rem_euclid(MICROS_PER_HOUR);
            (l, l + MICROS_PER_HOUR)
        }
        Cadence::Daily(h, m) => {
            let l = at(last_day, h, m);
            if l <= end { (l, l + MICROS_PER_DAY) } else { (l - MICROS_PER_DAY, l) }
        }
        Cadence::Weekly(wd, h) => {
            let mut d = last_day;
            while time::date_of(d).weekday().num_days_from_monday() != wd {
                d -= 1;
            }
            let l = at(d, h, 0);
            (l, l + 7 * MICROS_PER_DAY)
        }
        Cadence::Monthly(h) => {
            let d = time::date_of(last_day);
            let first = chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap();
            let l = time::ts_of_date(first) + h * MICROS_PER_HOUR;
            (l, time::ts_of_date(first.checked_add_months(chrono::Months::new(1)).unwrap()) + h * MICROS_PER_HOUR)
        }
    }
}

impl MerchantSim {
    pub fn audit(&mut self, rng: &mut Rng, a: Audit) -> Option<Rid> {
        let tenant = self.tenant_rid();
        audit(&mut self.chunk, rng, tenant, a)
    }

    fn staff_with(&self, roles: &[&str]) -> Option<usize> {
        self.staff.iter().position(|s| roles.contains(&s.role))
    }

    /// A staff member with one of the roles (in order of preference), with their IP and browser.
    pub fn staff_actor(&self, roles: &[&str]) -> Option<(Rid, String, &'static str)> {
        let i = roles.iter().find_map(|r| self.staff.iter().position(|s| s.role == *r))?;
        let m = self.admin.staff.get(i)?;
        Some((self.staff[i].user.rid, m.ip.clone(), m.ua))
    }

    pub(super) fn setup_admin(&mut self, day: Day, joined: Ts, rng: &mut Rng, refs: &Refs) {
        let tenant = self.tenant_rid();
        let end = time::end_ts();
        self.admin.office_ip = people::ip(rng);
        // roles and their permissions
        let mut names: Vec<&'static str> = vec!["Owner", "Administrator", "Support Agent", "Warehouse", "Marketing", "Merchandiser", "Finance"];
        if self.b2b || self.m.size_rank <= 20 || self.staff.iter().any(|s| s.role == "sales") {
            names.push("Sales Rep");
        }
        let custom = rng.chance(0.3);
        if custom {
            names.push("Read-only");
        }
        for name in names {
            let mut r = Row::new(roles::NCOLS);
            r.set(roles::TENANT_ID, tenant)
                .set(roles::NAME, name)
                .set(roles::DESCRIPTION, if name == "Read-only" { V::Static("View-only access for agencies and auditors") } else { V::Null })
                .set(roles::IS_SYSTEM, name != "Read-only")
                .set(roles::CREATED_AT, V::Ts(joined))
                .set(roles::UPDATED_AT, V::Ts(joined));
            let rid = self.chunk.emit(roles::T, day, r);
            self.admin.roles.insert(name, rid);
            let perms = role_permissions(name);
            for (code, prid) in &refs.permissions {
                let all = perms.is_empty() && (name == "Owner" || !code.starts_with("billing.write"));
                if all || perms.contains(code) {
                    let mut r = Row::new(role_permissions::NCOLS);
                    r.set(role_permissions::TENANT_ID, tenant).set(role_permissions::ROLE_ID, rid).set(role_permissions::PERMISSION_ID, *prid).set(role_permissions::CREATED_AT, V::Ts(joined));
                    self.chunk.emit(role_permissions::T, day, r);
                }
            }
        }
        // staff: invitation, role, team, sign-in methods
        let owner = self.staff[0].user.rid;
        let mut teams: BTreeMap<&str, Rid> = BTreeMap::new();
        let mut team = |c: &mut Chunk, name: &'static str, parent: Option<Rid>| -> Rid {
            if let Some(r) = teams.get(name) {
                return *r;
            }
            let mut r = Row::new(teams::NCOLS);
            r.set(teams::TENANT_ID, tenant).set(teams::NAME, name).set(teams::PARENT_TEAM_ID, V::opt_ref(parent)).set(teams::CREATED_AT, V::Ts(joined)).set(teams::UPDATED_AT, V::Ts(joined));
            let rid = c.emit(teams::T, day, r);
            teams.insert(name, rid);
            rid
        };
        let root = team(&mut self.chunk, "Leadership", None);
        let nested = self.m.size_rank <= 30;
        for i in 0..self.staff.len() {
            let (h, role) = (self.staff[i].user, self.staff[i].role);
            let (created, email) = {
                let u = self.chunk.deferred_ref(h);
                (u.vals[users::CREATED_AT].as_ts().unwrap(), match &u.vals[users::EMAIL] { V::Str(s) => s.clone(), _ => String::new() })
            };
            let role_rid = self.admin.roles[role_name(role)];
            if i > 0 {
                let sent = created - rng.int(1, 72) * MICROS_PER_HOUR;
                let mut r = Row::new(invitations::NCOLS);
                r.set(invitations::TENANT_ID, tenant)
                    .set(invitations::EMAIL, email.clone())
                    .set(invitations::ROLE_ID, role_rid)
                    .set(invitations::INVITED_BY_USER_ID, owner)
                    .set(invitations::STATUS, "accepted")
                    .set(invitations::TOKEN_HASH, rng.hex(64))
                    .set(invitations::EXPIRES_AT, V::Ts(sent + 7 * MICROS_PER_DAY))
                    .set(invitations::ACCEPTED_AT, V::Ts(created))
                    .set(invitations::CREATED_AT, V::Ts(sent));
                self.chunk.emit(invitations::T, day, r);
            }
            let mut r = Row::new(user_roles::NCOLS);
            r.set(user_roles::TENANT_ID, tenant).set(user_roles::USER_ID, h.rid).set(user_roles::ROLE_ID, role_rid).set(user_roles::GRANTED_BY_USER_ID, if i == 0 { V::Null } else { V::Ref(owner) }).set(user_roles::CREATED_AT, V::Ts(created));
            self.chunk.emit(user_roles::T, day, r);
            if custom && rng.chance(0.15) && i > 0 {
                let mut r = Row::new(user_roles::NCOLS);
                r.set(user_roles::TENANT_ID, tenant).set(user_roles::USER_ID, h.rid).set(user_roles::ROLE_ID, self.admin.roles["Read-only"]).set(user_roles::GRANTED_BY_USER_ID, owner).set(user_roles::CREATED_AT, V::Ts(created + MICROS_PER_DAY));
                self.chunk.emit(user_roles::T, day, r);
            }
            let tname: &'static str = match role {
                "owner" | "admin" => "Leadership",
                "support" => "Customer Service",
                "warehouse" => "Warehouse",
                "marketing" | "merchandiser" => "Marketing & Merchandising",
                "finance" => "Finance",
                _ => "Sales",
            };
            let parent = if nested && tname == "Warehouse" { Some(team(&mut self.chunk, "Operations", Some(root))) } else if tname == "Leadership" { None } else { Some(root) };
            let t = team(&mut self.chunk, tname, parent);
            let mut r = Row::new(team_members::NCOLS);
            r.set(team_members::TENANT_ID, tenant).set(team_members::TEAM_ID, t).set(team_members::USER_ID, h.rid).set(team_members::TEAM_ROLE, if i == 0 || (role == "support" && !self.staff[..i].iter().any(|s| s.role == "support")) { "lead" } else { "member" }).set(team_members::CREATED_AT, V::Ts(created));
            self.chunk.emit(team_members::T, day, r);
            let x = rng.f64();
            if x < 0.45 || (x < 0.53) {
                let mut r = Row::new(mfa_devices::NCOLS);
                r.set(mfa_devices::TENANT_ID, tenant)
                    .set(mfa_devices::USER_ID, h.rid)
                    .set(mfa_devices::KIND, if x < 0.45 { "totp" } else { "webauthn" })
                    .set(mfa_devices::SECRET_HASH, rng.hex(64))
                    .set(mfa_devices::CONFIRMED_AT, if rng.chance(0.95) { V::Ts(created + rng.int(1, 600) * MICROS_PER_SEC) } else { V::Null })
                    .set(mfa_devices::CREATED_AT, V::Ts(created));
                self.chunk.emit(mfa_devices::T, day, r);
            }
            if rng.chance(0.25) {
                let mut r = Row::new(oauth_accounts::NCOLS);
                r.set(oauth_accounts::TENANT_ID, tenant).set(oauth_accounts::USER_ID, h.rid).set(oauth_accounts::PROVIDER, "google").set(oauth_accounts::PROVIDER_ACCOUNT_ID, rng.digits(21)).set(oauth_accounts::ACCESS_TOKEN_HASH, rng.hex(64)).set(oauth_accounts::CREATED_AT, V::Ts(created));
                self.chunk.emit(oauth_accounts::T, day, r);
            }
            let home = rng.chance(0.3);
            self.admin.staff.push(StaffMeta { ip: if home { people::ip(rng) } else { self.admin.office_ip.clone() }, ua: people::user_agent(rng, people::Device::Desktop), created });
        }
        // invitations that were never accepted
        for _ in 0..rng.poisson(1.0) {
            let sent = joined + rng.int(30, 1000) * MICROS_PER_DAY;
            if sent > end {
                continue;
            }
            let p = people::person(rng, crate::world::world().country(&self.m.country));
            let mut r = Row::new(invitations::NCOLS);
            let expires = sent + 7 * MICROS_PER_DAY;
            r.set(invitations::TENANT_ID, tenant)
                .set(invitations::EMAIL, format!("{}.{}@{}", people::ascii_fold(&p.first).to_lowercase().replace(' ', ""), people::ascii_fold(&p.last).to_lowercase().replace(' ', ""), self.domain))
                .set(invitations::ROLE_ID, self.admin.roles["Support Agent"])
                .set(invitations::INVITED_BY_USER_ID, owner)
                .set(invitations::STATUS, if expires > end { "pending" } else if rng.chance(0.3) { "revoked" } else { "expired" })
                .set(invitations::TOKEN_HASH, rng.hex(64))
                .set(invitations::EXPIRES_AT, V::Ts(expires))
                .set(invitations::CREATED_AT, V::Ts(sent));
            let d = time::day_of_ts(sent).max(day as i64) as Day;
            self.chunk.emit(invitations::T, d, r);
        }
        let admin = self.staff_with(&["admin", "owner"]).map(|i| self.staff[i].user.rid);
        if crate::sim::stories::s13_erp(self.m) {
            // S13: the ERP connector update that started writing local timestamps
            let at = time::ts_of_date(chrono::NaiveDate::from_ymd_opt(2024, 10, 26).unwrap()) + 16 * MICROS_PER_HOUR;
            let mut r = Row::new(tenant_settings::NCOLS);
            r.set(tenant_settings::TENANT_ID, tenant).set(tenant_settings::SETTING_KEY, "integrations.erp.timestamp_mode").set(tenant_settings::SETTING_VALUE, V::Json("\"local\"".to_string())).set(tenant_settings::CREATED_AT, V::Ts(joined)).set(tenant_settings::UPDATED_AT, V::Ts(at));
            let st = self.chunk.emit(tenant_settings::T, day, r);
            let ip = self.admin.office_ip.clone();
            let a = self.audit(rng, Audit { at, action: "update", actor: admin, actor_type: "staff", entity: Some((tenant_settings::T, st)), summary: "Upgraded ERP connector to v3".into(), metadata: r#"{"connector":"erp_connector","version":{"from":"2.7.4","to":"3.0.0"}}"#.into(), ip: Some(&ip), ua: None });
            let tenant = self.tenant_rid();
            change(&mut self.chunk, tenant, a, admin, (tenant_settings::T, st), &["setting_value"], r#"{"setting_value":"utc"}"#.into(), r#"{"setting_value":"local"}"#.into(), at);
        }
        // settings
        let s = self.m.size_rank;
        for (key, val) in [
            ("store.timezone", format!("\"{}\"", self.m.timezone)),
            ("store.currency", format!("\"{}\"", self.m.currency)),
            ("checkout.guest_checkout", if rng.chance(0.6) { "true".into() } else { "false".into() }),
            ("orders.number_prefix", format!("\"{}\"", self.order_prefix)),
            ("tax.prices_include_tax", (self.m.country != "US").to_string()),
            ("shipping.free_threshold", format!(r#"{{"amount":"{}.00","currency":"{}"}}"#, self.free_shipping_threshold() / 100, self.m.currency)),
            ("notifications.sender", format!(r#"{{"name":{},"email":"hello@{}"}}"#, crate::sim::shopper::json_str(&self.m.name), self.domain)),
            ("analytics.sample_rate", "0.15".into()),
        ] {
            let mut r = Row::new(tenant_settings::NCOLS);
            r.set(tenant_settings::TENANT_ID, tenant).set(tenant_settings::SETTING_KEY, key).set(tenant_settings::SETTING_VALUE, V::Json(val)).set(tenant_settings::CREATED_AT, V::Ts(joined)).set(tenant_settings::UPDATED_AT, V::Ts(joined + rng.int(0, 300) * MICROS_PER_DAY));
            self.chunk.emit(tenant_settings::T, day, r);
        }
        for (key, val, ty, secret, desc) in [
            ("store_name", crate::sim::shopper::json_str(&self.m.name), "string", false, "Name shown in the storefront and emails"),
            ("returns_window_days", if self.b2b { "14".into() } else { "30".into() }, "number", false, "Days after delivery in which returns are accepted"),
            ("reviews_enabled", (!self.b2b).to_string(), "boolean", false, "Show product reviews"),
            ("low_stock_threshold", format!("{}", 3 + s % 5), "number", false, "Alert when stock falls below"),
            ("launch_date", format!("\"{}\"", self.m.joined_on), "date", false, "First day on the platform"),
            ("smtp_password", "\"********\"".into(), "string", true, "Credentials for the custom sender domain"),
            ("checkout_fields", r#"{"phone":"optional","company":"hidden"}"#.into(), "json", false, "Checkout form configuration"),
        ] {
            let mut r = Row::new(settings::NCOLS);
            r.set(settings::TENANT_ID, tenant)
                .set(settings::SETTING_KEY, key)
                .set(settings::SETTING_VALUE, V::Json(val))
                .set(settings::VALUE_TYPE, ty)
                .set(settings::IS_SECRET, secret)
                .set(settings::DESCRIPTION, if rng.chance(0.8) { V::Static(desc) } else { V::Null })
                .set(settings::CREATED_AT, V::Ts(joined))
                .set(settings::UPDATED_AT, V::Ts(joined));
            self.chunk.emit(settings::T, day, r);
        }
        // integrations with their service accounts, API keys, secrets and rate limits
        let mut key_n = 0;
        let mut api_key = |this: &mut MerchantSim, rng: &mut Rng, name: &str, at: Ts| -> Rid {
            key_n += 1;
            let mut code = String::new();
            let mut x = (this.idx * 32 + key_n) as u64 + 1296;
            while x > 0 {
                code.insert(0, char::from_digit((x % 36) as u32, 36).unwrap());
                x /= 36;
            }
            let revoked = rng.chance(0.1);
            let mut r = Row::new(api_keys::NCOLS);
            r.set(api_keys::PUBLIC_ID, V::Uuid(rng.uuid()))
                .set(api_keys::TENANT_ID, this.tenant_rid())
                .set(api_keys::NAME, name.to_string())
                .set(api_keys::PREFIX, format!("tk_live_{code}"))
                .set(api_keys::HASHED_KEY, rng.hex(64))
                .set(api_keys::STATUS, if revoked { "revoked" } else { "active" })
                .set(api_keys::CREATED_BY_USER_ID, V::opt_ref(admin))
                .set(api_keys::LAST_USED_AT, V::Ts(if revoked { at + rng.int(30, 300) * MICROS_PER_DAY } else { end - rng.int(60, 7200) * MICROS_PER_SEC }.min(end)))
                .set(api_keys::CREATED_AT, V::Ts(at))
                .set(api_keys::UPDATED_AT, V::Ts(at));
            this.chunk.emit(api_keys::T, day, r)
        };
        let systems: Vec<(&str, &'static str, &'static str, i64)> = self.webhooks.iter().map(|w| w.events.clone()).enumerate().map(|(i, _)| match i {
            0 => ("ERP", "erp_connector", "bidirectional", if s <= 5 { 6 } else if s <= 10 { 12 } else { 24 }),
            1 => ("Warehouse management", "wms_connector", "inbound", if s <= 5 { 6 } else if s <= 10 { 12 } else { 24 }),
            _ => ("Accounting", "accounting_connector", "outbound", 24),
        }).collect();
        let mut systems = systems;
        if s <= 15 {
            systems.push(("Marketplace product feed", "marketplace_feed", "outbound", 24));
        }
        for (name, provider, direction, every_h) in systems {
            let at = joined + rng.int(1, 40) * MICROS_PER_DAY;
            let mut r = Row::new(service_accounts::NCOLS);
            r.set(service_accounts::TENANT_ID, tenant).set(service_accounts::NAME, format!("{name} sync")).set(service_accounts::DESCRIPTION, format!("Machine user for the {} integration", name.to_lowercase())).set(service_accounts::CREATED_AT, V::Ts(at)).set(service_accounts::UPDATED_AT, V::Ts(at));
            self.chunk.emit(service_accounts::T, day, r);
            let key = api_key(self, rng, &format!("{name} sync"), at);
            let error = provider == "erp_connector" && self.webhooks.first().is_some_and(|w| w.failure_rate > 0.1);
            let mut r = Row::new(integrations::NCOLS);
            r.set(integrations::TENANT_ID, tenant)
                .set(integrations::PROVIDER, provider)
                .set(integrations::NAME, name)
                .set(integrations::STATUS, if error { "error" } else { "connected" })
                .set(integrations::CONFIG, V::Json(format!(r#"{{"endpoint":"https://{}.{}/api","sync_interval_minutes":{},"objects":["orders","products","stock"],"timezone":"UTC"}}"#, provider.split('_').next().unwrap(), self.domain, every_h * 60)))
                .set(integrations::CONNECTED_BY_USER_ID, V::opt_ref(admin))
                .set(integrations::CONNECTED_AT, V::Ts(at))
                .set(integrations::LAST_SYNCED_AT, V::Ts(end - rng.int(1, every_h * 3600) * MICROS_PER_SEC))
                .set(integrations::LAST_ERROR, if error { V::Static("504 Gateway Timeout from remote endpoint") } else { V::Null })
                .set(integrations::CREATED_AT, V::Ts(at))
                .set(integrations::UPDATED_AT, V::Ts(end - rng.int(1, 30) * MICROS_PER_DAY));
            let rid = self.chunk.emit(integrations::T, day, r);
            self.admin.integrations.push(Integration { rid, every_h, direction, kind: provider });
            self.admin.health.push((provider, Some(rid)));
            let rotations = ((end - at) / (365 * MICROS_PER_DAY)).max(0);
            let mut r = Row::new(secrets_vault::NCOLS);
            r.set(secrets_vault::TENANT_ID, tenant)
                .set(secrets_vault::KEY, format!("{}_api_token", provider.split('_').next().unwrap()))
                .set(secrets_vault::PROVIDER, "platform_vault")
                .set(secrets_vault::EXTERNAL_REF, format!("vault://tenants/{}/{}#v{}", self.m.slug, provider, rotations + 1))
                .set(secrets_vault::VERSION, V::Int(rotations + 1))
                .set(secrets_vault::LAST_ROTATED_AT, if rotations > 0 { V::Ts(at + rotations * 365 * MICROS_PER_DAY) } else { V::Null })
                .set(secrets_vault::EXPIRES_AT, V::Ts(at + (rotations + 1) * 365 * MICROS_PER_DAY))
                .set(secrets_vault::CREATED_BY_USER_ID, V::opt_ref(admin))
                .set(secrets_vault::CREATED_AT, V::Ts(at))
                .set(secrets_vault::UPDATED_AT, V::Ts(at + rotations * 365 * MICROS_PER_DAY));
            self.chunk.emit(secrets_vault::T, day, r);
            let mut r = Row::new(rate_limits::NCOLS);
            r.set(rate_limits::TENANT_ID, tenant).set(rate_limits::API_KEY_ID, key).set(rate_limits::LIMIT_SCOPE, "admin_api").set(rate_limits::MAX_REQUESTS, 120).set(rate_limits::WINDOW_SECONDS, 60).set(rate_limits::IS_ENABLED, true).set(rate_limits::CREATED_AT, V::Ts(at)).set(rate_limits::UPDATED_AT, V::Ts(at));
            self.chunk.emit(rate_limits::T, day, r);
        }
        for (scope, max, win) in [("storefront_api", 600, 60), ("admin_api", 2000, 3600), ("checkout", 30, 60)] {
            let mut r = Row::new(rate_limits::NCOLS);
            r.set(rate_limits::TENANT_ID, tenant).set(rate_limits::LIMIT_SCOPE, scope).set(rate_limits::MAX_REQUESTS, max).set(rate_limits::WINDOW_SECONDS, win).set(rate_limits::IS_ENABLED, true).set(rate_limits::CREATED_AT, V::Ts(joined)).set(rate_limits::UPDATED_AT, V::Ts(joined));
            self.chunk.emit(rate_limits::T, day, r);
        }
        for (key, prov) in [("smtp_credentials", "platform_vault"), ("payment_webhook_secret", "merchant_kms")] {
            let mut r = Row::new(secrets_vault::NCOLS);
            r.set(secrets_vault::TENANT_ID, tenant)
                .set(secrets_vault::KEY, key)
                .set(secrets_vault::PROVIDER, prov)
                .set(secrets_vault::EXTERNAL_REF, format!("{}://{}/{key}", if prov == "platform_vault" { "vault" } else { "kms" }, self.m.slug))
                .set(secrets_vault::VERSION, 1)
                .set(secrets_vault::CREATED_BY_USER_ID, V::opt_ref(admin))
                .set(secrets_vault::CREATED_AT, V::Ts(joined))
                .set(secrets_vault::UPDATED_AT, V::Ts(joined));
            self.chunk.emit(secrets_vault::T, day, r);
        }
        if s <= 40 {
            for i in 0..self.staff.len().min(3) {
                if !matches!(self.staff[i].role, "owner" | "admin") {
                    continue;
                }
                let created = self.admin.staff[i].created + rng.int(1, 200) * MICROS_PER_DAY;
                if created > end {
                    continue;
                }
                let mut r = Row::new(personal_access_tokens::NCOLS);
                r.set(personal_access_tokens::TENANT_ID, tenant)
                    .set(personal_access_tokens::USER_ID, self.staff[i].user.rid)
                    .set(personal_access_tokens::NAME, *rng.pick(&["reporting script", "CLI", "inventory sheet", "bulk price update"]))
                    .set(personal_access_tokens::HASHED_TOKEN, rng.hex(64))
                    .set(personal_access_tokens::SCOPES, V::Arr(rng.pick(&[vec!["orders:read", "products:read"], vec!["products:write"], vec!["reports:read"]]).iter().map(|x| x.to_string()).collect()))
                    .set(personal_access_tokens::LAST_USED_AT, if rng.chance(0.7) { V::Ts(end - rng.int(1, 90) * MICROS_PER_DAY) } else { V::Null })
                    .set(personal_access_tokens::EXPIRES_AT, if rng.chance(0.5) { V::Ts(created + 365 * MICROS_PER_DAY) } else { V::Null })
                    .set(personal_access_tokens::CREATED_AT, V::Ts(created));
                let d = time::day_of_ts(created).max(day as i64) as Day;
                self.chunk.emit(personal_access_tokens::T, d, r);
            }
        }
        if s <= 30 {
            api_key(self, rng, "Storefront app", joined + MICROS_PER_DAY);
        }
        self.admin.health.push(("storefront", None));
        self.admin.health.push(("checkout", None));
        // feature flags
        for (key, name, kind, enabled, default, rollout) in [
            ("new_checkout", "New one-page checkout", "percentage_rollout", true, "false", Some(rng.int(0, 10) * 10)),
            ("product_reviews", "Product reviews widget", "boolean", !self.b2b, "true", None),
            ("recommendations_algo", "Recommendation algorithm", "multivariate", true, "\"bought_together\"", None),
            ("express_shipping", "Express shipping option", "boolean", rng.chance(0.6), "false", None),
        ] {
            let mut r = Row::new(feature_flags::NCOLS);
            r.set(feature_flags::TENANT_ID, tenant)
                .set(feature_flags::KEY, key)
                .set(feature_flags::NAME, name)
                .set(feature_flags::DESCRIPTION, V::Null)
                .set(feature_flags::KIND, kind)
                .set(feature_flags::IS_ENABLED, enabled)
                .set(feature_flags::DEFAULT_VALUE, V::Json(default.to_string()))
                .set(feature_flags::ROLLOUT_PERCENTAGE, rollout.map_or(V::Null, V::Int))
                .set(feature_flags::CREATED_AT, V::Ts(joined))
                .set(feature_flags::UPDATED_AT, V::Ts(joined + rng.int(0, 500) * MICROS_PER_DAY));
            let flag = self.chunk.emit(feature_flags::T, day, r);
            if kind != "boolean" && rng.chance(0.6) {
                let (cond, serve) = if kind == "multivariate" { (r#"{"device":{"in":["mobile"]}}"#, "\"trending\"") } else { (r#"{"country":{"in":["DE","AT","CH"]}}"#, "true") };
                let mut r = Row::new(feature_flag_rules::NCOLS);
                r.set(feature_flag_rules::TENANT_ID, tenant).set(feature_flag_rules::FEATURE_FLAG_ID, flag).set(feature_flag_rules::SORT_POSITION, 0).set(feature_flag_rules::CONDITIONS, V::Json(cond.to_string())).set(feature_flag_rules::SERVE_VALUE, V::Json(serve.to_string())).set(feature_flag_rules::IS_ACTIVE, true).set(feature_flag_rules::CREATED_AT, V::Ts(joined)).set(feature_flag_rules::UPDATED_AT, V::Ts(joined));
                self.chunk.emit(feature_flag_rules::T, day, r);
            }
            if rng.chance(0.3) {
                let u = self.staff[rng.below(self.staff.len())].user.rid;
                let mut r = Row::new(feature_flag_overrides::NCOLS);
                r.set(feature_flag_overrides::TENANT_ID, tenant).set(feature_flag_overrides::FEATURE_FLAG_ID, flag).set(feature_flag_overrides::USER_ID, u).set(feature_flag_overrides::OVERRIDE_VALUE, V::Json(if kind == "multivariate" { "\"trending\"".into() } else { "true".into() })).set(feature_flag_overrides::REASON, V::Static("staff beta tester")).set(feature_flag_overrides::CREATED_AT, V::Ts(joined)).set(feature_flag_overrides::UPDATED_AT, V::Ts(joined));
                self.chunk.emit(feature_flag_overrides::T, day, r);
            }
        }
        // background jobs and their schedules
        let mut defs: Vec<(&str, &str, &str, Cadence, bool, f64)> = vec![
            ("nightly_segment_refresh", "marketing", "segments.refresh", Cadence::Daily(2, 17), false, 4000.0),
            ("payout_reconciliation", "finance", "payouts.reconcile", Cadence::Daily(6, 30), false, 2500.0),
            ("abandoned_cart_reminders", "marketing", "carts.remind", Cadence::Hourly, false, 800.0),
            ("sitemap_generation", "storefront", "sitemap.build", Cadence::Weekly(0, 4), false, 15000.0),
            ("search_reindex", "storefront", "search.reindex", Cadence::Weekly(6, 3), false, 60000.0),
            ("audit_log_archival", "compliance", "audit.archive", Cadence::Monthly(3), false, 30000.0),
        ];
        if self.mkt.loyalty.is_some() {
            defs.push(("loyalty_points_release", "loyalty", "loyalty.release", Cadence::Daily(2, 0), true, 1500.0));
        }
        if self.subs.is_some() {
            defs.push(("subscription_renewals", "billing", "subscriptions.renew", Cadence::Daily(0, 30), false, 5000.0));
        }
        for (name, queue, handler, cadence, local, mean_ms) in defs {
            let mut r = Row::new(jobs::NCOLS);
            r.set(jobs::TENANT_ID, tenant)
                .set(jobs::NAME, name)
                .set(jobs::QUEUE, queue)
                .set(jobs::HANDLER, handler)
                .set(jobs::PAYLOAD, V::Json(format!(r#"{{"tenant":"{}"}}"#, self.m.slug)))
                .set(jobs::MAX_ATTEMPTS, 3)
                .set(jobs::IS_ENABLED, true)
                .set(jobs::CREATED_AT, V::Ts(joined))
                .set(jobs::UPDATED_AT, V::Ts(joined));
            let rid = self.chunk.emit(jobs::T, day, r);
            let cron = match cadence {
                Cadence::Daily(h, m) => format!("{m} {h} * * *"),
                Cadence::Weekly(wd, h) => format!("0 {h} * * {}", (wd + 1) % 7),
                Cadence::Monthly(h) => format!("0 {h} 1 * *"),
                Cadence::Hourly => "0 * * * *".to_string(),
            };
            let (last, next) = last_next(cadence, &self.tz, local);
            let mut r = Row::new(scheduled_tasks::NCOLS);
            r.set(scheduled_tasks::TENANT_ID, tenant)
                .set(scheduled_tasks::NAME, name)
                .set(scheduled_tasks::CRON_EXPRESSION, cron)
                .set(scheduled_tasks::TIMEZONE, if local { self.m.timezone.clone() } else { "UTC".to_string() })
                .set(scheduled_tasks::JOB_ID, rid)
                .set(scheduled_tasks::IS_ENABLED, self.churn_day.is_none())
                .set(scheduled_tasks::LAST_RUN_AT, V::Ts(last))
                .set(scheduled_tasks::NEXT_RUN_AT, if self.churn_day.is_none() { V::Ts(next) } else { V::Null })
                .set(scheduled_tasks::CREATED_AT, V::Ts(joined))
                .set(scheduled_tasks::UPDATED_AT, V::Ts(last));
            self.chunk.emit(scheduled_tasks::T, day, r);
            self.admin.jobs.push(Job { rid, cadence, local, mean_ms, fail: if name == "search_reindex" { 0.05 } else { 0.01 } });
        }
        // retention policies (the email log policy was only drafted recently)
        for (table_, days, action, active) in [
            (events::T, 1095, "delete", true),
            (page_views::T, 1095, "delete", true),
            (job_runs::T, 90, "delete", true),
            (system_health_checks::T, 35, "delete", true),
            (audit_log::T, 730, "archive", true),
            (email_log::T, 730, "delete", false),
        ] {
            let created = if active { joined } else { time::ts_of_day(time::num_days() - 40) };
            let mut r = Row::new(retention_policies::NCOLS);
            r.set(retention_policies::TENANT_ID, tenant)
                .set(retention_policies::NAME, format!("{} retention", table(table_).name.replace('_', " ")))
                .set(retention_policies::ENTITY_SCHEMA, table(table_).module.clone())
                .set(retention_policies::ENTITY_TABLE, table(table_).name.clone())
                .set(retention_policies::RETENTION_DAYS, days)
                .set(retention_policies::ACTION_ON_EXPIRY, action)
                .set(retention_policies::IS_ACTIVE, active)
                .set(retention_policies::LAST_RUN_AT, if active && self.churn_day.is_none() { V::Ts(time::ts_of_day(time::num_days() - 1) + 3 * MICROS_PER_HOUR) } else { V::Null })
                .set(retention_policies::CREATED_AT, V::Ts(created.max(joined)))
                .set(retention_policies::UPDATED_AT, V::Ts(created.max(joined)));
            let d = time::day_of_ts(created).max(day as i64) as Day;
            self.chunk.emit(retention_policies::T, d, r);
        }
        // the catalog migrated from the previous platform came in as an import
        if joined < time::ts_of_day(0) {
            let at = time::ts_of_day(0) - rng.int(2, 20) * MICROS_PER_DAY;
            self.import(rng, day, at, "products", (self.launch_rate * 1096.0 / 0.35 * 0.65) as i64);
        }
        let logo_at = joined + MICROS_PER_HOUR;
        let (size, versions) = (rng.int(4_000, 40_000), 1 + rng.below(3) as i64);
        self.file(rng, day, logo_at, &format!("{}-logo.svg", self.m.slug), "image/svg+xml", size, "tidewick-media", admin, versions);
    }

    /// S16: after the credential-stuffing attack: MFA for staff, a login rate limit, a new API
    /// key, and a compliance case.
    fn s16_response(&mut self, rng: &mut Rng, day: Day) {
        let tenant = self.tenant_rid();
        let at = time::ts_of_day(day) + 8 * MICROS_PER_HOUR;
        let admin = self.staff_with(&["admin", "owner"]).map(|i| self.staff[i].user.rid);
        for i in 0..self.staff.len() {
            if !rng.chance(0.6) {
                continue;
            }
            let t = at + rng.int(0, 8 * 24) * MICROS_PER_HOUR;
            let mut r = Row::new(mfa_devices::NCOLS);
            r.set(mfa_devices::TENANT_ID, tenant).set(mfa_devices::USER_ID, self.staff[i].user.rid).set(mfa_devices::KIND, "totp").set(mfa_devices::SECRET_HASH, rng.hex(64)).set(mfa_devices::CONFIRMED_AT, V::Ts(t + rng.int(30, 600) * MICROS_PER_SEC)).set(mfa_devices::CREATED_AT, V::Ts(t));
            let d = time::day_of_ts(t) as Day;
            self.chunk.emit(mfa_devices::T, d, r);
        }
        let mut r = Row::new(rate_limits::NCOLS);
        r.set(rate_limits::TENANT_ID, tenant).set(rate_limits::LIMIT_SCOPE, "login").set(rate_limits::MAX_REQUESTS, 5).set(rate_limits::WINDOW_SECONDS, 300).set(rate_limits::IS_ENABLED, true).set(rate_limits::CREATED_AT, V::Ts(at)).set(rate_limits::UPDATED_AT, V::Ts(at));
        let rl = self.chunk.emit(rate_limits::T, day, r);
        let mut r = Row::new(api_keys::NCOLS);
        r.set(api_keys::PUBLIC_ID, V::Uuid(rng.uuid()))
            .set(api_keys::TENANT_ID, tenant)
            .set(api_keys::NAME, "Storefront app (rotated after incident)")
            .set(api_keys::PREFIX, format!("tk_live_s16{}", self.idx))
            .set(api_keys::HASHED_KEY, rng.hex(64))
            .set(api_keys::STATUS, "active")
            .set(api_keys::CREATED_BY_USER_ID, V::opt_ref(admin))
            .set(api_keys::LAST_USED_AT, V::Ts(time::end_ts() - rng.int(60, 3600) * MICROS_PER_SEC))
            .set(api_keys::CREATED_AT, V::Ts(at + MICROS_PER_HOUR))
            .set(api_keys::UPDATED_AT, V::Ts(at + MICROS_PER_HOUR));
        self.chunk.emit(api_keys::T, day, r);
        let ip = self.admin.office_ip.clone();
        self.audit(rng, Audit { at, action: "update", actor: admin, actor_type: "staff", entity: Some((rate_limits::T, rl)), summary: "Enabled login rate limiting (5 attempts / 5 min)".into(), metadata: r#"{"reason":"credential stuffing"}"#.into(), ip: Some(&ip), ua: None });
        let mut r = Row::new(compliance_events::NCOLS);
        r.set(compliance_events::TENANT_ID, tenant)
            .set(compliance_events::EVENT_TYPE, "credential_stuffing")
            .set(compliance_events::SEVERITY, "high")
            .set(compliance_events::DETECTED_BY_USER_ID, V::opt_ref(admin))
            .set(compliance_events::DETAILS, V::Json(r#"{"window":"2025-02-08/2025-02-11","source":"botnet","actions":["forced password resets","staff MFA","login rate limit","API key rotation"]}"#.to_string()))
            .set(compliance_events::RESOLVED, true)
            .set(compliance_events::RESOLVED_AT, V::Ts(at + 9 * MICROS_PER_DAY))
            .set(compliance_events::OCCURRED_AT, V::Ts(at - 14 * MICROS_PER_HOUR))
            .set(compliance_events::CREATED_AT, V::Ts(at));
        self.chunk.emit(compliance_events::T, day, r);
    }

    #[allow(clippy::too_many_arguments)]
    fn file(&mut self, rng: &mut Rng, day: Day, at: Ts, name: &str, ctype: &str, size: i64, bucket: &str, by: Option<Rid>, versions: i64) -> Option<Rid> {
        let end = time::end_ts();
        if at > end {
            return None;
        }
        let tenant = self.tenant_rid();
        let public = rng.uuid();
        let key = format!("tenants/{}/{}/{}", self.m.slug, crate::rng::format_uuid(public), name);
        let mut r = Row::new(files::NCOLS);
        r.set(files::PUBLIC_ID, V::Uuid(public))
            .set(files::TENANT_ID, tenant)
            .set(files::FILENAME, name.to_string())
            .set(files::CONTENT_TYPE, ctype.to_string())
            .set(files::BYTE_SIZE, V::Int(size))
            .set(files::STORAGE_BUCKET, bucket.to_string())
            .set(files::STORAGE_KEY, key.clone())
            .set(files::CHECKSUM, format!("sha256:{}", rng.hex(64)))
            .set(files::UPLOADED_BY_USER_ID, V::opt_ref(by))
            .set(files::CREATED_AT, V::Ts(at))
            .set(files::UPDATED_AT, V::Ts(at));
        let d = time::day_of_ts(at).max(day as i64) as Day;
        let f = self.chunk.emit(files::T, d, r);
        let mut t = at;
        for v in 1..=versions {
            if t > end {
                break;
            }
            let next = t + rng.int(20, 400) * MICROS_PER_DAY;
            let current = v == versions || next > end;
            let mut r = Row::new(file_versions::NCOLS);
            r.set(file_versions::TENANT_ID, tenant)
                .set(file_versions::FILE_ID, f)
                .set(file_versions::VERSION, V::Int(v))
                .set(file_versions::BYTE_SIZE, V::Int((size as f64 * rng.lognormal(0.0, 0.2)) as i64))
                .set(file_versions::STORAGE_KEY, format!("{key}.v{v}"))
                .set(file_versions::CHECKSUM, format!("sha256:{}", rng.hex(64)))
                .set(file_versions::IS_CURRENT, current)
                .set(file_versions::CREATED_BY_USER_ID, V::opt_ref(by))
                .set(file_versions::CREATED_AT, V::Ts(t))
                .set(file_versions::UPDATED_AT, V::Ts(t));
            let d = time::day_of_ts(t).max(day as i64) as Day;
            self.chunk.emit(file_versions::T, d, r);
            if current {
                break;
            }
            t = next;
        }
        Some(f)
    }

    pub(super) fn import_customers(&mut self, rng: &mut Rng, day: Day, at: Ts, rows: i64) {
        self.import(rng, day, at, "customers", rows);
    }

    /// A CSV import with a few rejected rows.
    fn import(&mut self, rng: &mut Rng, day: Day, at: Ts, entity: &str, rows: i64) {
        let tenant = self.tenant_rid();
        let by = self.staff_with(&["admin", "owner", "merchandiser"]).map(|i| self.staff[i].user.rid);
        let fname = format!("{entity}-export-{}.csv", chrono::DateTime::from_timestamp_micros(at).unwrap().format("%Y%m%d"));
        let size = rows * rng.int(180, 420);
        let file = self.file(rng, day, at, &fname, "text/csv", size, "tidewick-imports", by, 1);
        let failed = rng.poisson(rows as f64 * 0.004 + 0.5) as i64;
        let fin = at + (rows as f64 * rng.uniform(20.0, 60.0)) as i64 * 1000 + 5 * MICROS_PER_SEC;
        let mut r = Row::new(import_batches::NCOLS);
        r.set(import_batches::TENANT_ID, tenant)
            .set(import_batches::ENTITY_TYPE, entity.to_string())
            .set(import_batches::SOURCE_FILENAME, fname)
            .set(import_batches::FILE_ID, V::opt_ref(file))
            .set(import_batches::STATUS, if failed > 0 { "partially_failed" } else { "completed" })
            .set(import_batches::TOTAL_ROWS, rows + failed)
            .set(import_batches::PROCESSED_ROWS, rows + failed)
            .set(import_batches::SUCCEEDED_ROWS, rows)
            .set(import_batches::FAILED_ROWS, failed)
            .set(import_batches::STARTED_BY_USER_ID, V::opt_ref(by))
            .set(import_batches::STARTED_AT, V::Ts(at + MICROS_PER_SEC))
            .set(import_batches::FINISHED_AT, V::Ts(fin))
            .set(import_batches::CREATED_AT, V::Ts(at))
            .set(import_batches::UPDATED_AT, V::Ts(fin));
        let d = time::day_of_ts(at).max(day as i64) as Day;
        let batch = self.chunk.emit(import_batches::T, d, r);
        for _ in 0..failed {
            let row = rng.int(2, rows + failed + 1);
            let (col, code, msg, raw) = *rng.pick(&[
                ("price", "invalid_number", "Price '12,99' is not a valid decimal (use a dot)", r#"{"sku":"TMP-%","price":"12,99"}"#),
                ("sku", "duplicate", "SKU already exists", r#"{"sku":"%","name":"%"}"#),
                ("weight_grams", "out_of_range", "Weight must be positive", r#"{"sku":"%","weight_grams":"-1"}"#),
                ("category", "not_found", "Unknown category path", r#"{"sku":"%","category":"Misc > Other"}"#),
            ]);
            let raw = raw.replace('%', &format!("{}-{}", self.sku_prefix, rng.digits(5)));
            let mut r = Row::new(import_errors::NCOLS);
            r.set(import_errors::TENANT_ID, tenant)
                .set(import_errors::IMPORT_BATCH_ID, batch)
                .set(import_errors::SOURCE_ROW_NUMBER, V::Int(row))
                .set(import_errors::COLUMN_NAME, col)
                .set(import_errors::ERROR_CODE, code)
                .set(import_errors::ERROR_MESSAGE, msg)
                .set(import_errors::RAW_ROW, V::Json(raw))
                .set(import_errors::OCCURRED_AT, V::Ts(at + 2 * MICROS_PER_SEC))
                .set(import_errors::CREATED_AT, V::Ts(at + 2 * MICROS_PER_SEC));
            self.chunk.emit(import_errors::T, d, r);
        }
    }

    /// Daily back-office activity: staff logins, jobs, syncs, health checks, exports.
    pub(super) fn admin_day(&mut self, day: Day, rng: &mut Rng, orders_yesterday: u32) {
        use chrono::Datelike;
        let tenant = self.tenant_rid();
        let end = time::end_ts();
        let date = time::date_of(day);
        let wd = date.weekday().num_days_from_monday();
        let last_days = time::num_days() - day;
        // staff sign in every few working days (sessions last up to two weeks)
        for i in 0..self.staff.len() {
            let (h, role) = (self.staff[i].user, self.staff[i].role);
            if self.admin.staff[i].created > time::ts_of_day(day) || !rng.chance(if wd < 5 { 0.22 } else { 0.03 }) {
                continue;
            }
            if self.agents.iter().any(|a| a.user == h.rid && a.left.is_some_and(|l| l <= time::ts_of_day(day))) {
                continue;
            }
            let at = self.tz.local_to_utc(date, rng.int(7 * 60, 11 * 60) * MICROS_PER_MIN + rng.int(0, 59_999_999));
            let email = match &self.chunk.deferred_ref(h).vals[users::EMAIL] { V::Str(s) => s.clone(), _ => String::new() };
            let (ip, ua) = (self.admin.staff[i].ip.clone(), self.admin.staff[i].ua);
            let fails = if rng.chance(0.05) { 1 + rng.below(2) } else { 0 };
            for k in 0..=fails {
                let t = at - (fails - k) as i64 * rng.int(5, 40) * MICROS_PER_SEC;
                let mut r = Row::new(login_attempts::NCOLS);
                r.set(login_attempts::TENANT_ID, tenant).set(login_attempts::USER_ID, h.rid).set(login_attempts::EMAIL, email.clone()).set(login_attempts::SUCCEEDED, k == fails).set(login_attempts::IP_ADDRESS, ip.clone()).set(login_attempts::USER_AGENT, ua).set(login_attempts::OCCURRED_AT, V::Ts(t));
                self.chunk.emit(login_attempts::T, day, r);
            }
            let last_seen = (at + rng.int(20, 9 * 60) * MICROS_PER_MIN).min(end);
            let logout = rng.chance(0.3) && last_seen < end;
            let mut r = Row::new(sessions::NCOLS);
            r.set(sessions::TENANT_ID, tenant)
                .set(sessions::USER_ID, h.rid)
                .set(sessions::TOKEN_HASH, rng.hex(64))
                .set(sessions::IP_ADDRESS, ip.clone())
                .set(sessions::USER_AGENT, ua)
                .set(sessions::STARTED_AT, V::Ts(at))
                .set(sessions::LAST_SEEN_AT, V::Ts(last_seen))
                .set(sessions::EXPIRES_AT, V::Ts(at + 14 * MICROS_PER_DAY))
                .set(sessions::REVOKED_AT, if logout { V::Ts(last_seen) } else { V::Null });
            self.chunk.emit(sessions::T, day, r);
            self.chunk.deferred_mut(h).set(users::LAST_LOGIN_AT, V::Ts(at));
            let meta = format!(r#"{{"method":"password","mfa":{},"failed_attempts":{fails}}}"#, rng.chance(0.5));
            self.audit(rng, Audit { at, action: "login", actor: Some(h.rid), actor_type: "staff", entity: Some((users::T, h.rid)), summary: "Signed in to the admin".into(), metadata: meta, ip: Some(&ip), ua: Some(ua) });
            if logout {
                self.audit(rng, Audit { at: last_seen, action: "logout", actor: Some(h.rid), actor_type: "staff", entity: Some((users::T, h.rid)), summary: "Signed out".into(), metadata: "{}".into(), ip: Some(&ip), ua: Some(ua) });
            }
            if role == "warehouse" && rng.chance(0.03) {
                let t = at + rng.int(5, 120) * MICROS_PER_MIN;
                self.audit(rng, Audit { at: t, action: "access_denied", actor: Some(h.rid), actor_type: "staff", entity: None, summary: "Tried to open Settings > Payments".into(), metadata: r#"{"required_permission":"settings.write"}"#.into(), ip: Some(&ip), ua: Some(ua) });
            }
        }
        // monthly finance export of last month's orders
        if date.day() == 2 && day > 0 {
            if let Some(i) = self.staff_with(&["finance", "owner"]) {
                let h = self.staff[i].user.rid;
                let at = self.tz.local_to_utc(date, 9 * MICROS_PER_HOUR + rng.int(0, 7200) * MICROS_PER_SEC);
                let prev = date.checked_sub_months(chrono::Months::new(1)).unwrap();
                let name = format!("orders-{}.csv", prev.format("%Y-%m"));
                let n = (orders_yesterday as i64 * 30).max(1);
                let size = n * rng.int(250, 400);
                let file = self.file(rng, day, at, &name, "text/csv", size, "tidewick-exports", Some(h), 1);
                let ip = self.admin.staff[i].ip.clone();
                let ua = self.admin.staff[i].ua;
                if let Some(f) = file {
                    self.audit(rng, Audit { at, action: "export", actor: Some(h), actor_type: "staff", entity: Some((files::T, f)), summary: format!("Exported orders for {}", prev.format("%B %Y")), metadata: format!(r#"{{"format":"csv","rows":{n},"filters":{{"month":"{}"}}}}"#, prev.format("%Y-%m")), ip: Some(&ip), ua: Some(ua) });
                    let mut r = Row::new(access_log::NCOLS);
                    r.set(access_log::TENANT_ID, tenant)
                        .set(access_log::ACTOR_USER_ID, h)
                        .set(access_log::ENTITY_SCHEMA, table(orders::T).module.clone())
                        .set(access_log::ENTITY_TABLE, "orders")
                        .set(access_log::ACCESS_KIND, "export")
                        .set(access_log::FIELD_NAMES, V::Arr(["order_number", "customer_email", "billing_address_id", "grand_total"].iter().map(|x| x.to_string()).collect()))
                        .set(access_log::PURPOSE, "bookkeeping")
                        .set(access_log::IP_ADDRESS, ip)
                        .set(access_log::USER_AGENT, ua)
                        .set(access_log::OCCURRED_AT, V::Ts(at))
                        .set(access_log::CREATED_AT, V::Ts(at + 100_000));
                    self.chunk.emit(access_log::T, day, r);
                }
            }
        }
        if crate::sim::stories::s16_response(self.m, day) {
            self.s16_response(rng, day);
        }
        // job runs (kept for 90 days) and health checks (kept for 35 days)
        if last_days <= 90 {
            for j in 0..self.admin.jobs.len() {
                let (rid, cadence, local, mean_ms, fail) = { let x = &self.admin.jobs[j]; (x.rid, x.cadence, x.local, x.mean_ms, x.fail) };
                let at: Vec<(i64, i64)> = match cadence {
                    Cadence::Daily(h, m) => vec![(h, m)],
                    Cadence::Weekly(w, h) if w == wd => vec![(h, 0)],
                    Cadence::Monthly(h) if date.day() == 1 => vec![(h, 0)],
                    // hourly runs are only kept for the largest merchants (job history add-on)
                    Cadence::Hourly if self.m.size_rank <= 10 => (0..24).map(|h| (h, 0)).collect(),
                    _ => vec![],
                };
                for (h, m) in at {
                let queued = if local { self.tz.local_to_utc(date, h * MICROS_PER_HOUR + m * MICROS_PER_MIN) } else { time::ts_of_day(day) + h * MICROS_PER_HOUR + m * MICROS_PER_MIN };
                let mut t = queued;
                for attempt in 1..=3 {
                    let started = t + rng.int(100, 5000) * 1000;
                    let dur = (mean_ms * rng.lognormal(0.0, 0.5) * if orders_yesterday > 0 { 1.0 + (orders_yesterday as f64).ln() / 5.0 } else { 1.0 }) as i64;
                    let finished = started + dur * 1000;
                    if finished > end {
                        break;
                    }
                    // S3: during the platform outage jobs fail and retry
                    let down = crate::sim::stories::platform_down(started);
                    let failed = down || rng.chance(fail);
                    let mut r = Row::new(job_runs::NCOLS);
                    r.set(job_runs::TENANT_ID, tenant)
                        .set(job_runs::JOB_ID, rid)
                        .set(job_runs::STATUS, if failed { if attempt < 3 { "retrying" } else { "failed" } } else { "succeeded" })
                        .set(job_runs::ATTEMPT, attempt)
                        .set(job_runs::QUEUED_AT, V::Ts(t))
                        .set(job_runs::STARTED_AT, V::Ts(started))
                        .set(job_runs::FINISHED_AT, V::Ts(finished))
                        .set(job_runs::DURATION_MS, V::Int(dur))
                        .set(job_runs::ERROR_MESSAGE, if down { V::Static("503 Service Unavailable: storefront API") } else if failed { V::Static("TimeoutError: worker exceeded 300s") } else { V::Null })
                        .set(job_runs::RESULT_PAYLOAD, if failed { V::Null } else { V::Json(format!(r#"{{"processed":{}}}"#, rng.int(0, 5000))) })
                        .set(job_runs::CREATED_AT, V::Ts(t));
                    let d = time::day_of_ts(t).max(day as i64) as Day;
                    self.chunk.emit(job_runs::T, d, r);
                    if !failed {
                        break;
                    }
                    t = finished + if down { 20 } else { 5 } * MICROS_PER_MIN;
                }
                }
            }
        }
        if last_days <= 35 {
            for k in 0..self.admin.health.len() {
                let (component, integ) = self.admin.health[k];
                let flaky = integ.is_some() && self.webhooks.iter().any(|w| w.failure_rate > 0.1);
                for k in 0..12 {
                    // every two hours, on the hour (the outage on Black Friday shows at 14:00)
                    let at = time::ts_of_day(day) + 2 * k * MICROS_PER_HOUR + rng.int(0, 120) * MICROS_PER_SEC;
                    let healthy = !rng.chance(if flaky { 0.08 } else { 0.004 }) && !(crate::sim::stories::platform_down(at) && integ.is_none());
                    let latency = if healthy { rng.lognormal(libm::log(if component == "checkout" { 180.0 } else { 90.0 }), 0.4) as i64 } else { 10_000 };
                    let mut r = Row::new(system_health_checks::NCOLS);
                    r.set(system_health_checks::TENANT_ID, tenant)
                        .set(system_health_checks::COMPONENT, component)
                        .set(system_health_checks::IS_HEALTHY, healthy)
                        .set(system_health_checks::LATENCY_MS, V::Int(latency))
                        .set(system_health_checks::DETAILS, if healthy { V::Null } else { V::Json(r#"{"error":"timeout","threshold_ms":10000}"#.to_string()) })
                        .set(system_health_checks::CHECKED_AT, V::Ts(at))
                        .set(system_health_checks::CREATED_AT, V::Ts(at));
                    self.chunk.emit(system_health_checks::T, day, r);
                }
            }
        }
        // integration syncs
        for k in 0..self.admin.integrations.len() {
            let (rid, every_h, direction, kind) = { let x = &self.admin.integrations[k]; (x.rid, x.every_h, x.direction, x.kind) };
            if self.join_day > day {
                continue;
            }
            let flaky = kind == "erp_connector" && self.webhooks.first().is_some_and(|w| w.failure_rate > 0.1);
            let runs = 24 / every_h;
            for n in 0..runs {
                let started = time::ts_of_day(day) + (n * every_h) * MICROS_PER_HOUR + rng.int(0, 300) * MICROS_PER_SEC;
                let records = (orders_yesterday as f64 / runs as f64 * rng.lognormal(0.0, 0.3)).round() as i64 + if kind == "marketplace_feed" { self.live.len() as i64 } else { 0 };
                let failed = rng.chance(if flaky { 0.06 } else { 0.005 });
                let rec_failed = if failed { records } else { rng.poisson(records as f64 * 0.002) as i64 };
                let finished = started + (2000 + records * rng.int(20, 80)) * 1000;
                if finished > end {
                    break;
                }
                let mut r = Row::new(integration_syncs::NCOLS);
                r.set(integration_syncs::TENANT_ID, tenant)
                    .set(integration_syncs::INTEGRATION_ID, rid)
                    .set(integration_syncs::STATUS, if failed { "failed" } else { "succeeded" })
                    .set(integration_syncs::DIRECTION, direction)
                    .set(integration_syncs::RECORDS_PROCESSED, if failed { 0 } else { records })
                    .set(integration_syncs::RECORDS_FAILED, rec_failed)
                    .set(integration_syncs::STARTED_AT, V::Ts(started))
                    .set(integration_syncs::FINISHED_AT, V::Ts(finished))
                    .set(integration_syncs::ERROR_MESSAGE, if failed { V::Static("504 Gateway Timeout from remote endpoint") } else { V::Null })
                    .set(integration_syncs::CREATED_AT, V::Ts(started));
                self.chunk.emit(integration_syncs::T, day, r);
            }
        }
    }
}
