//! S8: a coupon-fraud ring. Its accounts share a handful of addresses, devices, cards and IP
//! addresses, take the welcome coupon on their only order and then claim it never arrived.
//! The merchant suspends them at the end of October and files a compliance event.

use super::ShopperChunk;
use crate::engine::DHandle;
use crate::rng::Rng;
use crate::sim::merchant::{Audit, MerchantSim, audit};
use crate::sim::people::{self, Address, Device};
use crate::tables::*;
use crate::time::{self, Day, MICROS_PER_HOUR, Ts};
use crate::value::{Row, V};

pub(super) struct Ring {
    pub addresses: Vec<Address>,
    /// shared devices: (anonymous id, device, user agent, profile row)
    pub devices: Vec<(u128, Device, &'static str, Option<DHandle>)>,
    pub cards: Vec<(&'static str, String)>,
    pub ips: Vec<String>,
}

impl ShopperChunk {
    pub(super) fn ring(&mut self, rng: &mut Rng, ms: &MerchantSim) -> &mut Ring {
        if self.ring.is_none() {
            let addresses = (0..20).map(|_| people::address(rng, ms.country)).collect();
            let devices = (0..4).map(|_| { let d = if rng.chance(0.5) { Device::Mobile } else { Device::Desktop }; (rng.uuid(), d, people::user_agent(rng, d), None) }).collect();
            let cards = (0..6).map(|_| (*rng.pick(&["visa", "mastercard"]), rng.digits(4))).collect();
            let base = format!("{}.{}.{}", rng.int(80, 200), rng.int(0, 255), rng.int(0, 255));
            let ips = (0..3).map(|i| format!("{base}.{}", 10 + i * 17)).collect();
            self.ring = Some(Ring { addresses, devices, cards, ips });
        }
        self.ring.as_mut().unwrap()
    }

    /// The ring's shared device profile for a new account (created with the first account).
    pub(super) fn ring_device(&mut self, rng: &mut Rng, ms: &MerchantSim, s: usize, at: Ts) {
        let k = rng.below(4);
        let existing = self.ring(rng, ms).devices[k].3;
        let h = match existing {
            Some(h) => h,
            None => {
                self.device_profile(rng, ms, s, 0, at);
                let h = self.shoppers[s].mkt.devices[0].unwrap();
                self.ring.as_mut().unwrap().devices[k].3 = Some(h);
                h
            }
        };
        self.shoppers[s].mkt.devices = [Some(h), Some(h)];
    }

    /// The ring is shut down: accounts suspended, a compliance case filed.
    pub(super) fn s8_crackdown(&mut self, rng: &mut Rng, ms: &MerchantSim, day: Day) {
        let tenant = ms.tenant_rid();
        let Some((staff, ip, ua)) = ms.staff_actor(&["admin", "owner"]) else { return };
        let mut n = 0;
        let mut t = time::ts_of_day(day) + 10 * MICROS_PER_HOUR;
        for s in 0..self.shoppers.len() {
            if !self.shoppers[s].fraud {
                continue;
            }
            n += 1;
            t += rng.int(5, 90) * 1_000_000;
            let (h, user) = (self.shoppers[s].user, self.shoppers[s].rid);
            self.chunk.deferred_mut(h).set(users::STATUS, "suspended").set(users::UPDATED_AT, V::Ts(t));
            self.shoppers[s].churn_day = self.shoppers[s].churn_day.min(day);
            let mut r2 = Rng::new(rng.u64(), &[8]);
            audit(&mut self.chunk, &mut r2, tenant, Audit { at: t, action: "update", actor: Some(staff), actor_type: "staff", entity: Some((users::T, user)), summary: "Suspended account: welcome coupon abuse".into(), metadata: r#"{"status":{"from":"active","to":"suspended"},"case":"coupon-fraud-2025-10"}"#.into(), ip: Some(&ip), ua: Some(ua) });
        }
        if n == 0 {
            return;
        }
        let mut r = Row::new(compliance_events::NCOLS);
        r.set(compliance_events::TENANT_ID, tenant)
            .set(compliance_events::EVENT_TYPE, "suspected_coupon_fraud")
            .set(compliance_events::SEVERITY, "high")
            .set(compliance_events::DETECTED_BY_USER_ID, staff)
            .set(compliance_events::DETAILS, V::Json(format!(r#"{{"accounts":{n},"coupon":"WELCOME10","shared_addresses":20,"shared_devices":4,"action":"accounts suspended"}}"#)))
            .set(compliance_events::RESOLVED, true)
            .set(compliance_events::RESOLVED_AT, V::Ts(t + MICROS_PER_HOUR))
            .set(compliance_events::OCCURRED_AT, V::Ts(time::ts_of_day(day) + 9 * MICROS_PER_HOUR))
            .set(compliance_events::CREATED_AT, V::Ts(time::ts_of_day(day) + 9 * MICROS_PER_HOUR));
        self.chunk.emit(compliance_events::T, day, r);
    }
}
