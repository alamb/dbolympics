//! Catalog metadata around products: category closure, attributes and variant attribute
//! values, media, tags, collections and bundles.

use std::collections::BTreeMap;

use super::MerchantSim;
use crate::engine::DHandle;
use crate::rng::Rng;
use crate::sim::people;
use crate::tables::*;
use crate::time::{self, Day, MICROS_PER_DAY, Ts};
use crate::value::{Rid, Row, V};
use crate::world::world;

#[derive(Default)]
pub struct CatalogMeta {
    /// variant axis -> (attribute, attribute value per axis value)
    pub attrs: BTreeMap<String, (Rid, Vec<Rid>)>,
    pub material: Option<Rid>,
    pub tags: Vec<(String, Rid)>,
    pub new_in: Option<Rid>,
    pub bestsellers: Option<Rid>,
    /// seasonal edits: (collection, season, first day, last day)
    pub seasonal: Vec<(Rid, i8, Day, Day)>,
    pub collection_pos: BTreeMap<Rid, i64>,
}

fn axis_label(axis: &str) -> (&'static str, &'static str, &'static str, Option<&'static str>) {
    // (code, name, kind, unit)
    match axis {
        a if a.starts_with("size") => ("size", "Size", "enum", None),
        a if a.starts_with("color") => ("colour", "Colour", "color", None),
        "storage" => ("storage", "Storage", "enum", None),
        "volume_ml" => ("volume", "Volume", "dimension", Some("ml")),
        "shade" => ("shade", "Shade", "color", None),
        "coffee_weight" | "pet_weight" => ("weight", "Weight", "dimension", Some("g")),
        "grind" => ("grind", "Grind", "enum", None),
        "pet_size" => ("pet_size", "Pet size", "enum", None),
        "pack" | "paper_pack" => ("pack_size", "Pack size", "enum", None),
        "ring_size" => ("ring_size", "Ring size", "enum", None),
        "metal" => ("metal", "Metal", "enum", None),
        "format_book" => ("format", "Format", "enum", None),
        "notebook_ruling" => ("ruling", "Ruling", "enum", None),
        "frame_size" => ("frame_size", "Frame size", "enum", Some("cm")),
        _ => ("option", "Option", "text", None),
    }
}

impl MerchantSim {
    /// Category closure, attributes, tags and evergreen collections (at onboarding).
    pub(super) fn setup_catalog_meta(&mut self, day: Day, ts: Ts, rng: &mut Rng) {
        let tenant = self.tenant_rid();
        // closure table: every (ancestor, descendant) pair along the category paths
        let cats: Vec<(String, Rid)> = self.categories.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (path, rid) in &cats {
            let parts: Vec<&str> = path.split(" > ").collect();
            for depth in 0..parts.len() {
                let anc = parts[..parts.len() - depth].join(" > ");
                let mut r = Row::new(category_closure::NCOLS);
                r.set(category_closure::TENANT_ID, tenant)
                    .set(category_closure::ANCESTOR_ID, self.categories[&anc])
                    .set(category_closure::DESCENDANT_ID, *rid)
                    .set(category_closure::DEPTH, depth)
                    .set(category_closure::CREATED_AT, V::Ts(ts));
                let h = self.chunk.emit_deferred(category_closure::T, day, r);
                self.closure_rows.insert((anc, path.clone()), h);
            }
        }
        // attributes from the variant axes of the product types sold
        let mut axes: Vec<String> = self.product_types.iter().flat_map(|pt| pt.axes.clone()).collect();
        axes.sort();
        axes.dedup();
        let mut by_code: BTreeMap<&str, Rid> = BTreeMap::new();
        let mut pos = 0;
        for ax in &axes {
            let (code, name, kind, unit) = axis_label(ax);
            let attr = match by_code.get(code) {
                Some(r) => *r,
                None => {
                    let mut r = Row::new(attributes::NCOLS);
                    r.set(attributes::TENANT_ID, tenant)
                        .set(attributes::CODE, code)
                        .set(attributes::NAME, name)
                        .set(attributes::KIND, kind)
                        .set(attributes::IS_VARIANT_DEFINING, true)
                        .set(attributes::IS_FILTERABLE, true)
                        .set(attributes::UNIT, unit.map(String::from))
                        .set(attributes::SORT_POSITION, pos)
                        .set(attributes::CREATED_AT, V::Ts(ts))
                        .set(attributes::UPDATED_AT, V::Ts(ts));
                    pos += 1;
                    let rid = self.chunk.emit(attributes::T, day, r);
                    by_code.insert(code, rid);
                    rid
                }
            };
            let mut vals = Vec::new();
            let values = &world().catalog.axes[ax];
            for (i, v) in values.iter().enumerate() {
                let mut r = Row::new(attribute_values::NCOLS);
                r.set(attribute_values::TENANT_ID, tenant)
                    .set(attribute_values::ATTRIBUTE_ID, attr)
                    .set(attribute_values::VALUE_CODE, format!("{}{}", people::slugify(v), if axes.iter().filter(|a| axis_label(a).0 == code).count() > 1 { format!("-{}", people::slugify(&ax.replace('_', " "))) } else { String::new() }))
                    .set(attribute_values::LABEL, v.clone())
                    .set(attribute_values::SORT_POSITION, i)
                    .set(attribute_values::CREATED_AT, V::Ts(ts))
                    .set(attribute_values::UPDATED_AT, V::Ts(ts));
                vals.push(self.chunk.emit(attribute_values::T, day, r));
            }
            self.meta.attrs.insert(ax.clone(), (attr, vals));
        }
        if self.product_types.iter().any(|pt| !pt.materials.is_empty()) {
            let mut r = Row::new(attributes::NCOLS);
            r.set(attributes::TENANT_ID, tenant)
                .set(attributes::CODE, "material")
                .set(attributes::NAME, "Material")
                .set(attributes::KIND, "text")
                .set(attributes::IS_VARIANT_DEFINING, false)
                .set(attributes::IS_FILTERABLE, true)
                .set(attributes::SORT_POSITION, pos)
                .set(attributes::CREATED_AT, V::Ts(ts))
                .set(attributes::UPDATED_AT, V::Ts(ts));
            self.meta.material = Some(self.chunk.emit(attributes::T, day, r));
        }
        for t in ["new", "bestseller", "sale", "eco", "limited-edition", "gift-idea", "exclusive", "back-in-stock", "last-chance", "vegan"] {
            if t == "vegan" && !matches!(self.m.vertical.as_str(), "beauty" | "food_coffee") {
                continue;
            }
            let mut r = Row::new(tags::NCOLS);
            r.set(tags::TENANT_ID, tenant).set(tags::NAME, if rng.chance(0.2) { t.to_uppercase() } else { t.replace('-', " ") }).set(tags::SLUG, t).set(tags::CREATED_AT, V::Ts(ts)).set(tags::UPDATED_AT, V::Ts(ts));
            self.meta.tags.push((t.to_string(), self.chunk.emit(tags::T, day, r)));
        }
        self.meta.new_in = Some(self.collection(day, ts, "New In", true, Some(r#"{"rules":[{"field":"published_at","op":"within_days","value":30}]}"#), None, None));
        self.meta.bestsellers = Some(self.collection(day, ts, "Bestsellers", true, Some(r#"{"rules":[{"field":"units_sold_30d","op":"top","value":50}]}"#), None, None));
    }

    #[allow(clippy::too_many_arguments)]
    fn collection(&mut self, day: Day, ts: Ts, name: &str, automated: bool, rules: Option<&str>, starts: Option<Ts>, ends: Option<Ts>) -> Rid {
        let mut r = Row::new(collections::NCOLS);
        r.set(collections::TENANT_ID, self.tenant_rid())
            .set(collections::NAME, name.to_string())
            .set(collections::SLUG, people::slugify(name))
            .set(collections::DESCRIPTION, if automated { V::Null } else { V::Str(format!("Our {} selection.", name.to_lowercase())) })
            .set(collections::IS_AUTOMATED, automated)
            .set(collections::RULES, rules.map_or(V::Null, |r| V::Json(r.to_string())))
            .set(collections::STARTS_AT, V::opt_ts(starts))
            .set(collections::ENDS_AT, V::opt_ts(ends))
            .set(collections::CREATED_AT, V::Ts(ts))
            .set(collections::UPDATED_AT, V::Ts(ts));
        self.chunk.emit(collections::T, day, r)
    }

    /// Seasonal edits, created a few weeks before each season.
    pub(super) fn seasonal_collections(&mut self, day: Day) {
        use chrono::Datelike;
        let d = time::date_of(day);
        if crate::sim::stories::has(self.m, "S18") && d == chrono::NaiveDate::from_ymd_opt(2024, 10, 28).unwrap() {
            let ts = time::ts_of_day(day) + 9 * crate::time::MICROS_PER_HOUR;
            let until = time::ts_of_date(chrono::NaiveDate::from_ymd_opt(2025, 1, 6).unwrap());
            let rid = self.collection(day, ts, "Gift Guide 2024", false, None, Some(ts), Some(until));
            let mut picks: Vec<(f64, Rid)> = self.products.iter().filter(|p| p.archive_day.is_none_or(|a| a > day) && p.launch_day <= day).map(|p| (p.appeal, p.rid)).collect();
            picks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            for (_, p) in picks.into_iter().take(40) {
                self.add_to_collection(day, ts, rid, p);
            }
            if let Some(gc) = self.gift_card {
                let p = self.products[gc].rid;
                self.add_to_collection(day, ts, rid, p);
            }
        }
        let (name, season, until) = match (d.month(), d.day()) {
            (10, 1) => (format!("Winter Edit {}/{}", d.year() % 100, (d.year() + 1) % 100), 1, d + chrono::Duration::days(150)),
            (4, 1) => (format!("Summer Edit {}", d.year()), 2, d + chrono::Duration::days(150)),
            _ => return,
        };
        let ts = time::ts_of_day(day) + 9 * crate::time::MICROS_PER_HOUR;
        let end_ts = time::ts_of_date(until);
        let rid = self.collection(day, ts, &name, false, None, Some(ts), Some(end_ts));
        self.meta.seasonal.push((rid, season, day, (day as i64 + 150) as Day));
        // seed it with the live products of the season
        let picks: Vec<Rid> = self.products.iter().filter(|p| p.season == season && p.archive_day.is_none_or(|a| a > day)).map(|p| p.rid).take(60).collect();
        for p in picks {
            self.add_to_collection(day, ts, rid, p);
        }
    }

    /// S19: the category tree gets a "Clothing" level under each top category; the closure
    /// table is rebuilt (all its rows are recreated, so they carry the rebuild time).
    pub(super) fn restructure_categories(&mut self, day: Day) {
        let tenant = self.tenant_rid();
        let at = time::ts_of_day(day) + 21 * crate::time::MICROS_PER_HOUR;
        let tops: Vec<String> = self.categories.keys().filter(|p| !p.contains(" > ")).filter(|t| ["Women", "Men"].contains(&t.as_str())).cloned().collect();
        for top in tops {
            let leaves: Vec<String> = self.categories.keys().filter(|p| p.starts_with(&format!("{top} > ")) && p.matches(" > ").count() == 1).cloned().collect();
            if leaves.is_empty() {
                continue;
            }
            let mid = format!("{top} > Clothing");
            let mut r = Row::new(categories::NCOLS);
            r.set(categories::TENANT_ID, tenant)
                .set(categories::PARENT_ID, self.categories[&top])
                .set(categories::NAME, "Clothing")
                .set(categories::SLUG, crate::sim::people::slugify(&mid))
                .set(categories::SORT_POSITION, 0)
                .set(categories::IS_ACTIVE, true)
                .set(categories::CREATED_AT, V::Ts(at))
                .set(categories::UPDATED_AT, V::Ts(at));
            let h = self.chunk.emit_deferred(categories::T, day, r);
            self.categories.insert(mid.clone(), h.rid);
            self.category_rows.insert(mid.clone(), h);
            for (anc, desc, depth) in [(mid.clone(), mid.clone(), 0), (top.clone(), mid.clone(), 1)] {
                let mut r = Row::new(category_closure::NCOLS);
                r.set(category_closure::TENANT_ID, tenant).set(category_closure::ANCESTOR_ID, self.categories[&anc]).set(category_closure::DESCENDANT_ID, self.categories[&desc]).set(category_closure::DEPTH, depth).set(category_closure::CREATED_AT, V::Ts(at));
                let h = self.chunk.emit_deferred(category_closure::T, day, r);
                self.closure_rows.insert((anc, desc), h);
            }
            for leaf in leaves {
                let lh = self.category_rows[&leaf];
                self.chunk.deferred_mut(lh).set(categories::PARENT_ID, h.rid).set(categories::UPDATED_AT, V::Ts(at));
                let ch = self.closure_rows[&(top.clone(), leaf.clone())];
                self.chunk.deferred_mut(ch).set(category_closure::DEPTH, 2);
                let mut r = Row::new(category_closure::NCOLS);
                r.set(category_closure::TENANT_ID, tenant).set(category_closure::ANCESTOR_ID, h.rid).set(category_closure::DESCENDANT_ID, self.categories[&leaf]).set(category_closure::DEPTH, 1).set(category_closure::CREATED_AT, V::Ts(at));
                let nh = self.chunk.emit_deferred(category_closure::T, day, r);
                self.closure_rows.insert((mid.clone(), leaf.clone()), nh);
            }
        }
        // the rebuild recreated every closure row
        let all: Vec<DHandle> = self.closure_rows.values().copied().collect();
        for h in all {
            self.chunk.deferred_mut(h).set(category_closure::CREATED_AT, V::Ts(at));
        }
    }

    pub(super) fn add_to_collection(&mut self, day: Day, ts: Ts, coll: Rid, product: Rid) {
        let pos = {
            let p = self.meta.collection_pos.entry(coll).or_insert(0);
            *p += 1;
            *p
        };
        let mut r = Row::new(collection_products::NCOLS);
        r.set(collection_products::TENANT_ID, self.tenant_rid())
            .set(collection_products::COLLECTION_ID, coll)
            .set(collection_products::PRODUCT_ID, product)
            .set(collection_products::SORT_POSITION, V::Int(pos))
            .set(collection_products::CREATED_AT, V::Ts(ts));
        self.chunk.emit(collection_products::T, day, r);
    }

    /// Rows that describe a new product beyond `products` / `product_variants`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn product_meta(&mut self, rng: &mut Rng, day: Day, ts: Ts, prid: Rid, slug: &str, name: &str, category_path: &str, material: Option<&str>, season: i8, variants: &[(Rid, Vec<(String, usize)>)], images: usize) {
        let tenant = self.tenant_rid();
        // categories: the leaf and its parents
        let parts: Vec<&str> = category_path.split(" > ").collect();
        for k in (1..=parts.len()).rev() {
            let path = parts[..k].join(" > ");
            let mut r = Row::new(product_categories::NCOLS);
            r.set(product_categories::TENANT_ID, tenant)
                .set(product_categories::PRODUCT_ID, prid)
                .set(product_categories::CATEGORY_ID, self.categories[&path])
                .set(product_categories::SORT_POSITION, rng.int(0, 200))
                .set(product_categories::CREATED_AT, V::Ts(ts));
            self.chunk.emit(product_categories::T, day, r);
        }
        // media
        for k in 0..images {
            let video = k > 0 && rng.chance(0.06);
            let (wpx, hpx) = *rng.pick(&[(2000, 2000), (1600, 2000), (2400, 1600), (1200, 1200)]);
            let mut r = Row::new(product_media::NCOLS);
            r.set(product_media::TENANT_ID, tenant)
                .set(product_media::PRODUCT_ID, prid)
                .set(product_media::VARIANT_ID, if k > 0 && !variants.is_empty() && rng.chance(0.25) { V::Ref(variants[rng.below(variants.len())].0) } else { V::Null })
                .set(product_media::KIND, if video { "video" } else { "image" })
                .set(product_media::URL, format!("https://cdn.{}/products/{slug}-{}.{}", self.domain, k + 1, if video { "mp4" } else { "jpg" }))
                .set(product_media::ALT_TEXT, if rng.chance(0.55) { V::Str(name.to_string()) } else { V::Null })
                .set(product_media::SORT_POSITION, k)
                .set(product_media::IS_PRIMARY, k == 0)
                .set(product_media::WIDTH_PX, if video { V::Int(1920) } else { V::Int(wpx) })
                .set(product_media::HEIGHT_PX, if video { V::Int(1080) } else { V::Int(hpx) })
                .set(product_media::CREATED_AT, V::Ts(ts))
                .set(product_media::UPDATED_AT, V::Ts(ts));
            self.chunk.emit(product_media::T, day, r);
        }
        // variant attribute values
        for (vrid, axes) in variants {
            for (ax, vi) in axes {
                if let Some((attr, vals)) = self.meta.attrs.get(ax) {
                    let mut r = Row::new(variant_attribute_values::NCOLS);
                    r.set(variant_attribute_values::TENANT_ID, tenant)
                        .set(variant_attribute_values::VARIANT_ID, *vrid)
                        .set(variant_attribute_values::ATTRIBUTE_ID, *attr)
                        .set(variant_attribute_values::ATTRIBUTE_VALUE_ID, vals[*vi])
                        .set(variant_attribute_values::CREATED_AT, V::Ts(ts));
                    self.chunk.emit(variant_attribute_values::T, day, r);
                }
            }
            if let (Some(attr), Some(m)) = (self.meta.material, material) {
                let mut r = Row::new(variant_attribute_values::NCOLS);
                r.set(variant_attribute_values::TENANT_ID, tenant)
                    .set(variant_attribute_values::VARIANT_ID, *vrid)
                    .set(variant_attribute_values::ATTRIBUTE_ID, attr)
                    .set(variant_attribute_values::VALUE_TEXT, if rng.chance(0.15) { m.to_lowercase() } else { m.to_string() })
                    .set(variant_attribute_values::CREATED_AT, V::Ts(ts));
                self.chunk.emit(variant_attribute_values::T, day, r);
            }
        }
        // tags
        let n = rng.poisson(1.3) as usize;
        let mut used = Vec::new();
        for _ in 0..n.min(self.meta.tags.len()) {
            let k = rng.below(self.meta.tags.len());
            if used.contains(&k) {
                continue;
            }
            used.push(k);
            let mut r = Row::new(product_tags::NCOLS);
            r.set(product_tags::TENANT_ID, tenant).set(product_tags::PRODUCT_ID, prid).set(product_tags::TAG_ID, self.meta.tags[k].1).set(product_tags::CREATED_AT, V::Ts(ts));
            self.chunk.emit(product_tags::T, day, r);
        }
        // collections: new launches go to "New In"; seasonal products join the running edit
        if ts >= time::ts_of_day(0) - 30 * MICROS_PER_DAY {
            if let Some(c) = self.meta.new_in {
                self.add_to_collection(day, ts, c, prid);
            }
        }
        if season != 0 {
            if let Some(&(c, _, _, _)) = self.meta.seasonal.iter().rev().find(|(_, s, a, b)| *s == season && *a <= day && day <= *b) {
                self.add_to_collection(day, ts, c, prid);
            }
        }
    }

    /// A few curated bundles (sets) from existing products.
    pub(super) fn bundles(&mut self, rng: &mut Rng, day: Day, ts: Ts, n: usize) {
        let tenant = self.tenant_rid();
        for k in 0..n {
            if self.live.len() < 4 {
                return;
            }
            let base = self.live[rng.below(self.live.len())];
            let mut members = vec![base];
            let comps = self.complements[self.products[base].ty].clone();
            for _ in 0..(1 + rng.below(3)) {
                let ty = if comps.is_empty() { rng.below(self.product_types.len()) } else { comps[rng.below(comps.len())] };
                if let Some(pi) = self.product_of_type(rng, ty) {
                    if !members.contains(&pi) {
                        members.push(pi);
                    }
                }
            }
            if members.len() < 2 {
                continue;
            }
            let name = format!("{} {}", self.products[base].name, ["Set", "Bundle", "Starter Kit", "Duo", "Gift Set"][rng.below(5)]);
            let slug = format!("{}-bundle-{}", people::slugify(&self.products[base].name), self.product_seq as usize * 10 + k);
            let mut r = Row::new(product_bundles::NCOLS);
            r.set(product_bundles::TENANT_ID, tenant)
                .set(product_bundles::NAME, name.clone())
                .set(product_bundles::SLUG, slug)
                .set(product_bundles::DESCRIPTION, if rng.chance(0.5) { V::Str(format!("{name}: everything you need, together.")) } else { V::Null })
                .set(product_bundles::IS_ACTIVE, true)
                .set(product_bundles::CREATED_AT, V::Ts(ts))
                .set(product_bundles::UPDATED_AT, V::Ts(ts));
            let b = self.chunk.emit(product_bundles::T, day, r);
            for (pos, pi) in members.iter().enumerate() {
                let v = self.products[*pi].variants[0].rid;
                let mut r = Row::new(bundle_items::NCOLS);
                r.set(bundle_items::TENANT_ID, tenant)
                    .set(bundle_items::BUNDLE_ID, b)
                    .set(bundle_items::VARIANT_ID, v)
                    .set(bundle_items::QUANTITY, 1)
                    .set(bundle_items::SORT_POSITION, pos)
                    .set(bundle_items::CREATED_AT, V::Ts(ts))
                    .set(bundle_items::UPDATED_AT, V::Ts(ts));
                self.chunk.emit(bundle_items::T, day, r);
            }
        }
    }
}
