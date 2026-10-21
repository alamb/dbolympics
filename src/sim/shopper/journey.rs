//! Customer journeys: a session is a path through the shop (landing page, categories, search,
//! product pages, recommendations, cart, checkout) driven by the visitor's intent, channel,
//! device and taste. What is added to the cart is what gets bought.

use super::{Item, ShopperChunk, json_str, money_str};
use crate::rng::{Rng, Weighted, format_uuid};
use crate::sim::merchant::MerchantSim;
use crate::sim::people::{self, Device};
use crate::tables::*;
use crate::time::{MICROS_PER_SEC, Ts};
use crate::value::{Rid, Row, V};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Intent {
    Browse,
    Research,
    Buy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    Direct,
    Organic,
    PaidSearch,
    Social,
    PaidSocial,
    Email,
    Referral,
}

impl Channel {
    /// How the visitor first found the shop: the merchant's channel mix of the day (paid
    /// channels follow its spend).
    pub fn acquisition(rng: &mut Rng, mix: &[f64; 6]) -> Channel {
        use Channel::*;
        let mix = if mix.iter().sum::<f64>() > 0.0 { *mix } else { crate::sim::merchant::BASE_MIX };
        [Organic, PaidSearch, Social, PaidSocial, Referral, Direct][Weighted::new(mix).sample(rng)]
    }

    /// Without a running paid flight a paid click cannot happen: the visit comes unpaid.
    pub fn settle(self, ms: &MerchantSim) -> Channel {
        match self {
            Channel::PaidSearch if ms.mkt.paid[0].is_none() => Channel::Organic,
            Channel::PaidSocial if ms.mkt.paid[1].is_none() => Channel::Social,
            c => c,
        }
    }

    /// Channel of a later visit by a known shopper (habit or a mix).
    /// Paid search on later visits (brand searches, retargeting) follows the merchant's search
    /// spend; business buyers mostly come back directly.
    pub fn returning(rng: &mut Rng, habit: Channel, ms: &MerchantSim) -> Channel {
        use Channel::*;
        if rng.chance(0.35) {
            // people found through an ad mostly come back by searching the name or typing it
            return match habit {
                PaidSearch if !rng.chance(0.3) => Organic,
                PaidSocial if !rng.chance(0.3) => Direct,
                h => h,
            };
        }
        let paid = 12.0 * ms.mkt.acq_mix[1] / crate::sim::merchant::BASE_MIX[1] * if ms.b2b { 0.4 } else { 1.0 };
        // (email visits come from clicks in actual marketing emails)
        [Direct, Organic, PaidSearch, Social, Referral][Weighted::new([36.0, 30.0, paid, 12.0, 6.0]).sample(rng)]
    }

    /// Anonymous traffic: the day's mix, with a little more organic and social browsing.
    pub fn anonymous(rng: &mut Rng, mix: &[f64; 6]) -> Channel {
        use Channel::*;
        let mut w = if mix.iter().sum::<f64>() > 0.0 { *mix } else { crate::sim::merchant::BASE_MIX };
        w[0] *= 1.2;
        w[2] *= 1.15;
        w[5] *= 0.8;
        [Organic, PaidSearch, Social, PaidSocial, Referral, Direct][Weighted::new(w).sample(rng)]
    }

    fn conversion(self) -> f64 {
        match self {
            Channel::PaidSearch => 1.4,
            Channel::Email => 1.3,
            Channel::Direct => 1.2,
            Channel::Referral => 1.1,
            Channel::Organic => 1.0,
            Channel::PaidSocial => 0.7,
            Channel::Social => 0.6,
        }
    }

    fn bounce(self) -> f64 {
        match self {
            Channel::Social => 0.55,
            Channel::PaidSocial => 0.5,
            Channel::Organic | Channel::Referral => 0.4,
            Channel::PaidSearch => 0.35,
            Channel::Email | Channel::Direct => 0.3,
        }
    }
}

fn device_factor(d: Device) -> f64 {
    match d {
        Device::Desktop => 1.25,
        Device::Tablet => 1.0,
        Device::Mobile => 0.75,
    }
}

/// A shopper's latent traits: taste (preferred product types), own size, budget and how much
/// price and quality matter to them.
#[derive(Clone, Debug)]
pub struct Prefs {
    pub types: Vec<(usize, f64)>,
    /// position within size runs (0 = smallest, 1 = largest)
    pub size_pos: f64,
    /// relative budget (income level of the home country x personal variation)
    pub budget: f64,
    pub price_sens: f64,
    pub quality_sens: f64,
    /// tolerance for late deliveries and hassle (days-ish scale)
    pub patience: f64,
    pub home: usize,
    /// propensity to order several sizes and return the rest (1 = normal)
    pub bracket: f64,
}

impl Prefs {
    pub fn new(rng: &mut Rng, ntypes: usize, home: usize) -> Prefs {
        let k = 1 + rng.below(3.min(ntypes));
        let mut types: Vec<(usize, f64)> = Vec::new();
        while types.len() < k {
            let t = rng.below(ntypes);
            if !types.iter().any(|(x, _)| *x == t) {
                types.push((t, rng.lognormal(0.0, 0.7)));
            }
        }
        let budget = crate::world::world().geo.countries[home].income_index * rng.lognormal(0.0, 0.45);
        Prefs {
            types,
            size_pos: rng.gauss(0.5, 0.18).clamp(0.0, 1.0),
            budget,
            price_sens: rng.lognormal(0.0, 0.35) / libm::sqrt(budget),
            quality_sens: rng.lognormal(-0.2, 0.35),
            patience: rng.lognormal(0.7, 0.4),
            home,
            bracket: 1.0,
        }
    }

    fn taste(&self, ty: usize) -> f64 {
        if self.types.iter().any(|(t, _)| *t == ty) { 0.5 } else { 0.0 }
    }

    pub(super) fn pick_type(&self, rng: &mut Rng, ntypes: usize) -> usize {
        if rng.chance(0.3) || self.types.is_empty() {
            return rng.below(ntypes);
        }
        self.types[Weighted::new(self.types.iter().map(|t| t.1)).sample(rng)].0
    }
}

pub struct Visitor {
    pub user: Option<Rid>,
    pub anon: u128,
    pub device: Device,
    pub ua: &'static str,
    pub ip: String,
    pub channel: Channel,
    /// session rows are stored (consented, and sampled for anonymous traffic)
    pub tracked: bool,
    pub sample_rate: Option<f64>,
    /// utm_campaign and landing page path of a marketing email click
    pub campaign: Option<String>,
    pub landing: Option<String>,
    /// running experiment and variant
    pub exp: Option<(usize, usize)>,
    /// in the merchant's mobile app (S10)
    pub app: bool,
}

pub struct Journey {
    pub session: Option<Rid>,
    pub end: Ts,
    pub items: Vec<Item>,
    pub checkout: bool,
    pub cart_token: String,
    pub first_add: Ts,
    /// products wanted but out of stock
    pub oos: u32,
}

struct Page {
    path: String,
    title: String,
    at: Ts,
    dur: i64,
}

impl ShopperChunk {
    /// Build an order line for a product, choosing the variant by the shopper's size.
    /// Utility of a product for this shopper on its product page (logit scale, ~0 on average).
    pub(super) fn utility(&self, ms: &MerchantSim, pi: usize, prefs: &Prefs) -> f64 {
        let p = &ms.products[pi];
        let day = self.chunk.day;
        // current price (increases, markdowns) and any running promotion
        let now = libm::log(ms.price_factor(pi, day)) + ms.promo_for(pi, crate::time::ts_of_day(day) + 12 * crate::time::MICROS_PER_HOUR).map_or(0.0, |o| libm::log(1.0 - o.pct));
        let price_term = 0.6 * (p.price_rel + now) + 0.4 * (libm::log(p.price_eur / (ms.vertical_mid_eur * prefs.budget)) + now);
        // S19: no photos or size information: shoppers hesitate
        let sloppy = if p.sloppy { 1.3 } else { 0.0 };
        0.7 * prefs.quality_sens * p.quality + 0.8 * crate::sim::calendar::season_factor(p.season, self.chunk.day) + prefs.taste(p.ty) - prefs.price_sens * price_term - sloppy
    }

    /// Which product a shopper clicks on a list page: the most attractive of a few candidates
    /// (prices are visible on list pages).
    fn listed(&self, rng: &mut Rng, ms: &MerchantSim, ty: usize, prefs: &Prefs) -> Option<usize> {
        let mut best: Option<(f64, usize)> = None;
        for _ in 0..4 {
            let Some(pi) = ms.product_of_type(rng, ty) else { return None };
            let p = &ms.products[pi];
            let gumbel = -libm::log(-libm::log(rng.f64().max(1e-12)));
            let score = -0.5 * prefs.price_sens * p.price_rel + gumbel;
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, pi));
            }
        }
        best.map(|b| b.1)
    }

    /// The same product one size up or down (bracketing).
    fn bracket_item(&self, rng: &mut Rng, ms: &MerchantSim, pi: usize, it: &Item) -> Option<Item> {
        let (n, si) = it.size?;
        let prod = &ms.products[pi];
        let si2 = if si == 0 { 1 } else if si + 1 == n { si - 1 } else if rng.chance(0.5) { si + 1 } else { si - 1 };
        let inner = prod.variants.len() / n;
        let vidx = si2 * inner + (it.vidx % inner);
        let v = &prod.variants[vidx];
        Some(Item { size: Some((n, si2)), vidx, variant: v.rid, sku: v.sku.clone(), name: v.name.clone(), qty: 1, unit_gross: ms.price_now(pi, vidx, self.chunk.day), ..it.clone() })
    }

    pub(super) fn item_for(&self, rng: &mut Rng, ms: &MerchantSim, pi: usize, prefs: &Prefs) -> Item {
        let prod = &ms.products[pi];
        let nv = prod.variants.len();
        let mut size = None;
        let vi = if prod.sizes > 1 {
            // own size (sometimes one up or down, or a gift in another size); other axes free
            let n = prod.sizes;
            let mut si = (prefs.size_pos * (n - 1) as f64).round() as i64;
            if rng.chance(0.08) {
                si += if rng.chance(0.5) { 1 } else { -1 };
            }
            if rng.chance(0.03) {
                si = rng.below(n) as i64;
            }
            let si = si.clamp(0, n as i64 - 1) as usize;
            size = Some((n, si));
            let inner = nv / n;
            si * inner + rng.below(inner)
        } else if nv == 1 {
            0
        } else {
            rng.below(nv)
        };
        let v = &prod.variants[vi];
        // cheap items are bought in multiples
        let multi = 1.0 / (1.0 + libm::exp(1.2 + 0.9 * libm::log(prod.price_eur / 20.0)));
        let qty = if ms.b2b { (rng.lognormal(0.0, 0.9) * ms.vertical.quantity_scale).ceil().max(1.0) as i64 } else if rng.chance(multi) { 2 + rng.poisson(0.4) as i64 } else { 1 };
        let unit_gross = ms.price_now(pi, vi, self.chunk.day);
        Item { size, pidx: pi, vidx: vi, product: prod.rid, product_name: prod.name.clone(), variant: v.rid, sku: v.sku.clone(), name: v.name.clone(), qty, unit_gross, weight_g: v.weight_g }
    }

    fn context(&self, ms: &MerchantSim, v: &Visitor, path: &str) -> String {
        let (dev, os) = match v.device {
            Device::Desktop => ("desktop", if v.ua.contains("Macintosh") { "macOS" } else if v.ua.contains("Linux") { "Linux" } else { "Windows" }),
            Device::Mobile => ("mobile", if v.ua.contains("Android") { "Android" } else { "iOS" }),
            Device::Tablet => ("tablet", if v.ua.contains("Android") { "Android" } else { "iPadOS" }),
        };
        let sdk = crate::sim::stories::sdk_version(self.chunk.day, v.device != Device::Desktop && !v.app).0;
        let lib = if v.app { format!(r#"{{"name":"tidewick-mobile-sdk","version":"2.1.0"}},"platform":"{}""#, if os == "Android" { "android" } else { "ios" }) } else { format!(r#"{{"name":"tidewick-analytics.js","version":"{sdk}"}}"#) };
        let mut s = format!(
            r#""context":{{"device":{{"type":"{dev}","os":"{os}"}},"locale":"{}","library":{lib},"page":{{"path":{}}}}}"#,
            ms.m.locale,
            json_str(path)
        );
        if let Some(r) = v.sample_rate {
            s.push_str(&format!(r#","sample_rate":{r}"#));
        }
        s
    }

    /// Simulate one session. Emits web_sessions / page_views / events if `v.tracked`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn journey(&mut self, rng: &mut Rng, ms: &MerchantSim, v: &Visitor, prefs: &Prefs, intent: Intent, start: Ts, preset: Option<&[Item]>) -> Journey {
        let ntypes = ms.product_types.len();
        let mut pages: Vec<Page> = Vec::new();
        let mut events: Vec<(&'static str, String, Ts)> = Vec::new();
        let mut items: Vec<Item> = preset.map(|p| p.to_vec()).unwrap_or_default();
        let cart_token = rng.alnum(32);
        let mut first_add = start;
        let mut t = start;
        let lift = v.exp.map_or(1.0, |(e, k)| ms.analytics.experiments[e].variants[k].2);
        let conv = v.channel.conversion() * device_factor(v.device) * lift;
        let shop = &ms.m.name;
        let tracked = v.tracked;
        let page = |pages: &mut Vec<Page>, t: &mut Ts, rng: &mut Rng, path: String, title: String, mean_s: f64| {
            let dur = (rng.lognormal(libm::log(mean_s), 0.8) as i64).clamp(1, 1800);
            pages.push(Page { path, title, at: *t, dur });
            *t += (dur + rng.int(1, 4)) * MICROS_PER_SEC;
        };
        // untracked sessions still need the decisions and the clock, but no strings
        macro_rules! s {
            ($($e:tt)*) => { if tracked { format!($($e)*) } else { String::new() } };
        }
        let product_page = |this: &Self, pages: &mut Vec<Page>, events: &mut Vec<(&'static str, String, Ts)>, t: &mut Ts, rng: &mut Rng, pi: usize, list: &str, position: usize| {
            let p = &ms.products[pi];
            let at = *t;
            let dur = (rng.lognormal(libm::log(45.0), 0.8) as i64).clamp(2, 1800);
            *t += (dur + rng.int(1, 4)) * MICROS_PER_SEC;
            if !v.tracked {
                pages.push(Page { path: String::new(), title: String::new(), at, dur });
                return;
            }
            let path = format!("/products/{}", p.slug);
            pages.push(Page { path: path.clone(), title: format!("{} | {shop}", p.name), at, dur });
            let v0 = &p.variants[0];
            let props = format!(
                r#"{{"product":{{"id":"{}","sku":{},"name":{},"price":{},"currency":"{}"}},"list":"{list}","position":{position},{}}}"#,
                format_uuid(p.public_id),
                json_str(&v0.sku),
                json_str(&p.name),
                money_str(v0.price_cents),
                ms.m.currency,
                this.context(ms, v, &path)
            );
            events.push(("product_viewed", props, at + MICROS_PER_SEC));
        };

        if let (Some((e, k)), true) = (v.exp, tracked) {
            let ex = &ms.analytics.experiments[e];
            events.push(("experiment_viewed", format!(r#"{{"experiment":{{"key":"{}","variant":"{}"}},{}}}"#, ex.key, ex.variants[k].1, self.context(ms, v, "/")), start + 500_000));
        }
        // landing page by channel
        // S3: during the platform outage pages time out and visitors leave
        let down = crate::sim::stories::platform_down(start);
        let bounce = down || rng.chance(v.channel.bounce() * if intent == Intent::Buy { 0.2 } else { 1.0 });
        let mut last: Option<usize> = None;
        let landing_product = matches!(v.channel, Channel::PaidSearch | Channel::Social | Channel::PaidSocial | Channel::Referral) || (v.channel == Channel::Organic && rng.chance(0.5));
        if let Some(lp) = v.landing.as_deref() {
            let title = lp.rsplit('/').next().unwrap_or("").replace('-', " ");
            page(&mut pages, &mut t, rng, s!("{lp}"), s!("{title} | {shop}"), 25.0);
        } else if preset.is_some() {
            page(&mut pages, &mut t, rng, s!("/"), s!("{shop}"), 8.0);
        } else if landing_product {
            let ty = prefs.pick_type(rng, ntypes);
            if let Some(pi) = ms.product_of_type(rng, ty) {
                product_page(self, &mut pages, &mut events, &mut t, rng, pi, "landing", 1);
                last = Some(pi);
            }
        }
        if pages.is_empty() {
            if v.channel == Channel::Email || rng.chance(0.4) {
                let ty = prefs.pick_type(rng, ntypes);
                let cat = ms.product_types[ty].categories[0].split(" > ").last().unwrap();
                page(&mut pages, &mut t, rng, s!("/collections/{}", people::slugify(cat)), s!("{cat} | {shop}"), 30.0);
            } else {
                page(&mut pages, &mut t, rng, s!("/"), s!("{shop}"), 15.0);
            }
        }
        // browsing
        let steps = if bounce {
            0
        } else {
            match intent {
                Intent::Browse => rng.poisson(1.5) as usize,
                Intent::Research => 3 + rng.poisson(5.0) as usize,
                Intent::Buy => if preset.is_some() { 0 } else { 2 + rng.poisson(4.0) as usize },
            }
        };
        // intent sets the baseline odds of adding a product (logit scale)
        let add_offset = match intent {
            Intent::Buy => -0.85,
            Intent::Research => -2.44,
            Intent::Browse => -3.48,
        } + libm::log(conv);
        let mut force_complement = false;
        let mut oos = 0u32;
        for step in 0..steps {
            let r = rng.f64();
            let (pi, list) = if force_complement && last.is_some() {
                force_complement = false;
                let lp = last.unwrap();
                let rel = &ms.products[lp].related;
                if !rel.is_empty() && rng.chance(0.6) {
                    (Some(rel[rng.below(rel.len())]), "recommendation")
                } else {
                    let comps = &ms.complements[ms.products[lp].ty];
                    let ty = if comps.is_empty() { prefs.pick_type(rng, ntypes) } else { comps[rng.below(comps.len())] };
                    (ms.product_of_type(rng, ty), "cross_sell")
                }
            } else if last.is_some_and(|lp| !ms.products[lp].related.is_empty()) && r < 0.15 {
                let rel = &ms.products[last.unwrap()].related;
                (Some(rel[rng.below(rel.len())]), "recommendation")
            } else if r < 0.32 {
                let ty = prefs.pick_type(rng, ntypes);
                let pt = ms.product_types[ty];
                let q = if !pt.modifiers.is_empty() && rng.chance(0.4) { format!("{} {}", rng.pick(&pt.modifiers), pt.kind) } else { pt.kind.clone() }.to_lowercase();
                let n = ms.by_type.get(ty).map_or(0, |x| x.products.len());
                let results = if n == 0 { 0 } else { (n as f64 * rng.uniform(0.3, 1.0)) as usize };
                let path = s!("/search?q={}", q.replace(' ', "+"));
                let at = t;
                if tracked {
                    events.push(("search", format!(r#"{{"query":{},"results":{results},{}}}"#, json_str(&q), self.context(ms, v, &path)), at + MICROS_PER_SEC));
                }
                page(&mut pages, &mut t, rng, path, s!("Search: {q} | {shop}"), 12.0);
                if results == 0 {
                    continue;
                }
                (self.listed(rng, ms, ty, prefs), "search")
            } else if r < 0.45 {
                let ty = prefs.pick_type(rng, ntypes);
                let cat = ms.product_types[ty].categories[0].split(" > ").last().unwrap();
                page(&mut pages, &mut t, rng, s!("/collections/{}", people::slugify(cat)), s!("{cat} | {shop}"), 30.0);
                (self.listed(rng, ms, ty, prefs), "category")
            } else {
                let ty = match last {
                    Some(lp) if rng.chance(0.5) => ms.products[lp].ty,
                    _ => prefs.pick_type(rng, ntypes),
                };
                (self.listed(rng, ms, ty, prefs), "category")
            };
            let Some(pi) = pi else { continue };
            product_page(self, &mut pages, &mut events, &mut t, rng, pi, list, 1 + step % 24);
            last = Some(pi);
            let already = items.iter().any(|it| it.product == ms.products[pi].rid);
            let u = self.utility(ms, pi, prefs) + add_offset;
            let wants = !already && rng.chance(1.0 / (1.0 + libm::exp(-u)));
            let it = if wants { Some(self.item_for(rng, ms, pi, prefs)).filter(|it| ms.in_stock(pi, it.vidx)) } else { None };
            if wants && it.is_none() {
                oos += 1;
            }
            if let Some(it) = it {
                if items.is_empty() {
                    first_add = t;
                }
                if tracked { events.push((
                    "add_to_cart",
                    format!(
                        r#"{{"item":{{"sku":{},"product_id":"{}","quantity":{},"price":{}}},"cart":{{"token":"{cart_token}","items":{}}},{}}}"#,
                        json_str(&it.sku),
                        format_uuid(ms.products[pi].public_id),
                        it.qty,
                        money_str(it.unit_gross),
                        items.len() + 1,
                        self.context(ms, v, &pages.last().unwrap().path)
                    ),
                    t - MICROS_PER_SEC,
                )); }
                // unsure about the fit? order the neighbouring size too (and return one later)
                let bracket = ms.m.vertical == "apparel_footwear" && it.size.is_some() && rng.chance((0.14 * prefs.bracket * ms.products[pi].fit_uncert).min(0.9));
                let second = if bracket { self.bracket_item(rng, ms, pi, &it).filter(|b| ms.in_stock(pi, b.vidx)) } else { None };
                // coupon hunters take a third size as well
                let third = if bracket && prefs.bracket > 2.0 { self.bracket_item(rng, ms, pi, &it).filter(|b| ms.in_stock(pi, b.vidx) && second.as_ref().is_none_or(|s2| s2.vidx != b.vidx)) } else { None };
                items.push(it);
                if let Some(b) = second {
                    if tracked {
                        let ctx = self.context(ms, v, &pages.last().unwrap().path);
                        events.push(("add_to_cart", format!(r#"{{"item":{{"sku":{},"product_id":"{}","quantity":{},"price":{}}},"cart":{{"token":"{cart_token}","items":{}}},{ctx}}}"#, json_str(&b.sku), format_uuid(ms.products[pi].public_id), b.qty, money_str(b.unit_gross), items.len() + 1), t));
                    }
                    items.push(b);
                }
                if let Some(b) = third {
                    items.push(b);
                }
                force_complement = rng.chance(0.4);
            }
        }
        // decided to buy what they were looking at
        if intent == Intent::Buy && items.is_empty() {
            if let Some(lp) = last.filter(|_| rng.chance(0.85)) {
                let it = self.item_for(rng, ms, lp, prefs);
                if !ms.in_stock(lp, it.vidx) {
                    return self.finish_journey(ms, v, pages, events, start, t, items, false, cart_token, first_add, oos + 1);
                }
                first_add = t;
                if tracked {
                    let ctx = self.context(ms, v, &pages.last().unwrap().path);
                    events.push((
                        "add_to_cart",
                        format!(r#"{{"item":{{"sku":{},"product_id":"{}","quantity":{},"price":{}}},"cart":{{"token":"{cart_token}","items":1}},{ctx}}}"#, json_str(&it.sku), format_uuid(ms.products[lp].public_id), it.qty, money_str(it.unit_gross)),
                        t,
                    ));
                }
                items.push(it);
            }
        }
        // close to the free-shipping threshold? add a cheap extra rather than pay for shipping
        if !items.is_empty() && intent == Intent::Buy && prefs.home == ms.country {
            let gross: i64 = items.iter().map(|i| i.unit_gross * i.qty).sum();
            let gap = ms.free_shipping_threshold() - gross;
            if gap > 0 && gap <= 4 * ms.shipping_fee(0) && rng.chance(0.5) {
                let comps = &ms.complements[ms.products[items[0].pidx].ty];
                let mut best: Option<(i64, usize)> = None;
                for _ in 0..10 {
                    let ty = if !comps.is_empty() && rng.chance(0.7) { comps[rng.below(comps.len())] } else { rng.below(ntypes) };
                    if let Some(pi) = ms.product_of_type(rng, ty).filter(|pi| ms.in_stock(*pi, 0)) {
                        let price = ms.price_now(pi, 0, self.chunk.day);
                        if price >= gap && best.is_none_or(|(b, _)| price < b) && !items.iter().any(|it| it.product == ms.products[pi].rid) {
                            best = Some((price, pi));
                        }
                    }
                }
                if let Some((_, pi)) = best {
                    let mut it = self.item_for(rng, ms, pi, prefs);
                    it.qty = 1;
                    if tracked {
                        let ctx = self.context(ms, v, "/cart");
                        events.push(("add_to_cart", format!(r#"{{"item":{{"sku":{},"product_id":"{}","quantity":1,"price":{}}},"cart":{{"token":"{cart_token}","items":{}}},"reason":"free_shipping_threshold",{ctx}}}"#, json_str(&it.sku), format_uuid(ms.products[pi].public_id), money_str(it.unit_gross), items.len() + 1), t));
                    }
                    items.push(it);
                }
            }
        }
        let p_checkout = if items.is_empty() { 0.0 } else { (match intent { Intent::Buy => 0.87, Intent::Research => 0.1, Intent::Browse => 0.04 }) * libm::sqrt(conv) };
        let checkout = !down && rng.chance(p_checkout.min(0.97));
        if !items.is_empty() {
            page(&mut pages, &mut t, rng, s!("/cart"), s!("Your cart | {shop}"), 20.0);
        }
        if checkout && tracked {
            let value: i64 = items.iter().map(|i| i.unit_gross * i.qty).sum();
            let lines: Vec<String> = items.iter().map(|i| format!(r#"{{"sku":{},"quantity":{},"price":{}}}"#, json_str(&i.sku), i.qty, money_str(i.unit_gross))).collect();
            events.push((
                "checkout_started",
                format!(r#"{{"cart":{{"token":"{cart_token}"}},"value":{},"currency":"{}","items":[{}],{}}}"#, money_str(value), ms.m.currency, lines.join(","), self.context(ms, v, "/checkout/information")),
                t,
            ));
        }
        if checkout {
            for (path, title, secs) in [("/checkout/information", "Checkout", 40.0), ("/checkout/shipping", "Shipping", 15.0), ("/checkout/payment", "Payment", 35.0)] {
                page(&mut pages, &mut t, rng, s!("{path}"), s!("{title} | {shop}"), secs);
            }
        }
        self.finish_journey(ms, v, pages, events, start, t, items, checkout, cart_token, first_add, oos)
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_journey(&mut self, ms: &MerchantSim, v: &Visitor, pages: Vec<Page>, events: Vec<(&'static str, String, Ts)>, start: Ts, end: Ts, items: Vec<Item>, checkout: bool, cart_token: String, first_add: Ts, oos: u32) -> Journey {
        let session = if v.tracked { Some(self.emit_session(ms, v, &pages, &events, start, end)) } else { None };
        Journey { session, end, items, checkout, cart_token, first_add, oos }
    }

    fn emit_session(&mut self, ms: &MerchantSim, v: &Visitor, pages: &[Page], events: &[(&'static str, String, Ts)], start: Ts, end: Ts) -> Rid {
        let tenant = ms.tenant_rid();
        let (referrer, src, med, camp): (Option<String>, Option<&str>, Option<&str>, Option<String>) = match v.channel {
            Channel::Direct => (None, None, None, None),
            Channel::Organic => (Some("https://www.google.com/".into()), None, None, None),
            Channel::PaidSearch => (Some("https://www.google.com/".into()), Some("google"), Some("cpc"), ms.mkt.paid[0].as_ref().map(|p| p.1.clone())),
            Channel::Social => (Some("https://l.instagram.com/".into()), Some("instagram"), Some("social"), None),
            Channel::PaidSocial => (Some("https://www.facebook.com/".into()), Some("facebook"), Some("paid_social"), ms.mkt.paid[1].as_ref().map(|p| p.1.clone())),
            Channel::Email => (None, Some("newsletter"), Some("email"), v.campaign.clone()),
            Channel::Referral => (Some("https://www.trustedreviewsite.example/".into()), Some("referral"), None, None),
        };
        let camp = v.campaign.clone().or(camp);
        let (src, med) = if v.campaign.is_some() && v.channel == Channel::Social { (Some("instagram"), Some("influencer")) } else { (src, med) };
        let mut r = Row::new(web_sessions::NCOLS);
        r.set(web_sessions::TENANT_ID, tenant)
            .set(web_sessions::USER_ID, V::opt_ref(v.user))
            .set(web_sessions::ANONYMOUS_ID, V::Uuid(v.anon))
            .set(web_sessions::STARTED_AT, V::Ts(start))
            .set(web_sessions::ENDED_AT, V::Ts(end))
            .set(web_sessions::LANDING_PAGE, format!("https://{}{}", ms.domain, pages[0].path))
            .set(web_sessions::REFERRER, referrer.clone())
            .set(web_sessions::UTM_SOURCE, src.map(String::from))
            .set(web_sessions::UTM_MEDIUM, med.map(String::from))
            .set(web_sessions::UTM_CAMPAIGN, camp)
            .set(web_sessions::IP_ADDRESS, v.ip.clone())
            .set(web_sessions::USER_AGENT, v.ua.to_string())
            .set(web_sessions::IS_BOUNCE, pages.len() == 1)
            .set(web_sessions::PAGE_VIEW_COUNT, pages.len())
            .set(web_sessions::CREATED_AT, V::Ts(start))
            .set(web_sessions::UPDATED_AT, V::Ts(end));
        let sid = self.chunk.emit(web_sessions::T, self.chunk.day, r);
        {
            let o = self.outbox.entry(self.chunk.day).or_default();
            o.sessions += 1;
        }
        for (i, p) in pages.iter().enumerate() {
            let mut r = Row::new(page_views::NCOLS);
            r.set(page_views::TENANT_ID, tenant)
                .set(page_views::WEB_SESSION_ID, sid)
                .set(page_views::URL, format!("https://{}{}", ms.domain, p.path))
                .set(page_views::PATH, p.path.clone())
                .set(page_views::TITLE, p.title.clone())
                .set(page_views::REFERRER, if i == 0 { V::from(referrer.clone()) } else { V::Str(format!("https://{}{}", ms.domain, pages[i - 1].path)) })
                .set(page_views::DURATION_SECONDS, p.dur)
                .set(page_views::VIEWED_AT, V::Ts(p.at))
                .set(page_views::CREATED_AT, V::Ts(p.at));
            self.chunk.emit(page_views::T, self.chunk.day, r);
        }
        // S12: a broken SDK release sends every event twice
        let dup = crate::sim::stories::sdk_version(self.chunk.day, v.device != Device::Desktop && !v.app).1;
        for (i, (name, props, at)) in events.iter().enumerate() {
            self.event(tenant, v.user, sid, name, props.clone(), *at);
            if dup {
                self.event(tenant, v.user, sid, name, props.clone(), *at + (2 + (i as i64 * 3) % 7) * 1000);
            }
        }
        sid
    }
}

