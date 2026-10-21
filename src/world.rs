//! Static world data from world/*.json (embedded at compile time).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Share {
    pub name: String,
    pub share: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Merchant {
    #[allow(dead_code)] // part of the data model, not used by the simulator yet
    pub id: u32,
    pub slug: String,
    pub name: String,
    pub vertical: String,
    pub business_models: Vec<String>,
    pub country: String,
    pub currency: String,
    pub locale: String,
    pub timezone: String,
    pub size_rank: u32,
    pub employee_count: u32,
    pub signup_source: String,
    pub joined_on: String,
    pub churned_on: Option<String>,
    pub status: String,
    pub gateways: Vec<Share>,
    pub carriers: Vec<Share>,
    pub international_carriers: Vec<Share>,
    pub stories: Vec<String>,
}

impl Merchant {
    pub fn has_model(&self, m: &str) -> bool {
        self.business_models.iter().any(|x| x == m)
    }
}

#[derive(Deserialize)]
struct MerchantsDoc {
    merchants: Vec<Merchant>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VatRate {
    pub from: String,
    pub rate: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct City {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub postal: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Region {
    pub code: String,
    pub name: String,
    pub cities: Vec<City>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Country {
    pub iso2: String,
    pub iso3: String,
    pub name: String,
    pub phone_code: String,
    pub continent: String,
    #[allow(dead_code)] // part of the data model, not used by the simulator yet
    pub currency: String,
    pub vat: Vec<VatRate>,
    pub timezone: String,
    pub language: String,
    pub address_style: String,
    /// approximate relative income level (UK = 1.0)
    pub income_index: f64,
    pub regions: Vec<Region>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RateSpan {
    pub start: f64,
    pub end: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Currency {
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub minor_unit: i64,
    pub eur_rate: Option<RateSpan>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Locale {
    pub code: String,
    pub name: String,
    pub native_name: String,
}

#[derive(Deserialize)]
pub struct Geo {
    pub countries: Vec<Country>,
    pub currencies: Vec<Currency>,
    pub locales: Vec<Locale>,
    pub streets: BTreeMap<String, Vec<String>>,
    pub us_sales_tax: BTreeMap<String, f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Names {
    pub f: Vec<String>,
    pub m: Vec<String>,
    pub last: Vec<String>,
}

#[derive(Deserialize)]
pub struct People {
    pub names: BTreeMap<String, Names>,
    pub email_domains: BTreeMap<String, BTreeMap<String, f64>>,
    pub phone_formats: BTreeMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProductType {
    #[serde(rename = "type")]
    pub kind: String,
    pub categories: Vec<String>,
    pub price_eur: [f64; 2],
    pub weight_g: [f64; 2],
    pub axes: Vec<String>,
    pub modifiers: Vec<String>,
    pub materials: Vec<String>,
    #[serde(default)]
    pub complements: Vec<String>,
    /// "winter", "summer" or null
    pub season: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Vertical {
    pub label: String,
    #[allow(dead_code)] // part of the data model, not used by the simulator yet
    pub return_rate: f64,
    #[allow(dead_code)] // part of the data model, not used by the simulator yet
    pub conversion: f64,
    pub items_per_order: f64,
    #[serde(default = "one")]
    pub quantity_scale: f64,
    pub seasonality: [f64; 12],
    pub brand_style: String,
    #[serde(default)]
    pub brand_names: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)] // part of the data model, not used by the simulator yet
    pub subscription_share: f64,
    pub products: Vec<ProductType>,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize)]
pub struct Catalog {
    pub axes: BTreeMap<String, Vec<String>>,
    pub verticals: BTreeMap<String, Vertical>,
    pub price_levels: BTreeMap<String, f64>,
    pub price_endings: BTreeMap<String, Vec<f64>>,
}

pub struct World {
    pub merchants: Vec<Merchant>,
    pub geo: Geo,
    pub people: People,
    pub catalog: Catalog,
}

impl World {
    pub fn country(&self, iso2: &str) -> &Country {
        self.geo.countries.iter().find(|c| c.iso2 == iso2).unwrap_or_else(|| panic!("unknown country {iso2}"))
    }

    pub fn country_index(&self, iso2: &str) -> usize {
        self.geo.countries.iter().position(|c| c.iso2 == iso2).unwrap_or_else(|| panic!("unknown country {iso2}"))
    }

    pub fn vertical(&self, v: &str) -> &Vertical {
        &self.catalog.verticals[v]
    }

    /// Standard VAT/GST rate (percent) of a country on a date (YYYY-MM-DD compare).
    pub fn vat_rate(&self, country: &Country, date: &str) -> f64 {
        let mut r = country.vat[0].rate;
        for v in &country.vat {
            if v.from.as_str() <= date {
                r = v.rate;
            }
        }
        r
    }
}

/// All parcel carriers used by any merchant, sorted; their index identifies a carrier network.
pub fn carriers() -> &'static Vec<String> {
    static C: OnceLock<Vec<String>> = OnceLock::new();
    C.get_or_init(|| {
        let mut v: Vec<String> = world().merchants.iter().flat_map(|m| m.carriers.iter().chain(&m.international_carriers)).map(|c| c.name.clone()).collect();
        v.sort();
        v.dedup();
        v
    })
}

pub fn carrier_index(name: &str) -> usize {
    carriers().binary_search_by(|c| c.as_str().cmp(name)).unwrap_or_else(|_| panic!("unknown carrier {name}"))
}

pub fn world() -> &'static World {
    static W: OnceLock<World> = OnceLock::new();
    W.get_or_init(|| {
        let m: MerchantsDoc = serde_json::from_str(include_str!("../world/merchants.json")).expect("merchants.json");
        World {
            merchants: m.merchants,
            geo: serde_json::from_str(include_str!("../world/geo.json")).expect("geo.json"),
            people: serde_json::from_str(include_str!("../world/people.json")).expect("people.json"),
            catalog: serde_json::from_str(include_str!("../world/catalog.json")).expect("catalog.json"),
        }
    })
}
