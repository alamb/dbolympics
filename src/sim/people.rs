//! Synthetic people: names, emails, phones, addresses, devices.

use crate::rng::{Rng, Weighted};
use crate::world::{Country, world};

#[derive(Clone)]
pub struct Person {
    pub first: String,
    pub last: String,
}

/// Lowercase ASCII for email local parts and slugs (ä -> ae, ß -> ss, é -> e, ...).
pub fn ascii_fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let rep: &str = match ch {
            'ä' | 'Ä' | 'æ' | 'Æ' => "ae",
            'ö' | 'Ö' | 'ø' | 'Ø' => "oe",
            'ü' | 'Ü' => "ue",
            'ß' => "ss",
            'å' | 'Å' | 'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' => "a",
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'ě' => "e",
            'í' | 'ì' | 'î' | 'ï' | 'ı' | 'Í' => "i",
            'ó' | 'ò' | 'ô' | 'õ' | 'Ó' | 'ő' => "o",
            'ú' | 'ù' | 'û' | 'Ú' | 'ů' | 'ű' => "u",
            'ç' | 'č' | 'Č' => "c",
            'ñ' | 'ń' => "n",
            'ł' | 'Ł' => "l",
            'š' | 'ś' | 'Š' | 'ş' => "s",
            'ž' | 'ż' | 'ź' | 'Ž' => "z",
            'ř' => "r",
            'ý' => "y",
            'ğ' => "g",
            _ => "",
        };
        if !rep.is_empty() {
            out.push_str(rep);
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '.' {
            out.push(' ');
        }
    }
    out
}

pub fn slugify(s: &str) -> String {
    let f = ascii_fold(&s.replace('&', " and "));
    f.split_whitespace().collect::<Vec<_>>().join("-")
}

pub fn person(rng: &mut Rng, country: &Country) -> Person {
    let names = &world().people.names;
    let n = names.get(&country.language).unwrap_or(&names["en"]);
    let first = if rng.chance(0.5) { rng.pick(&n.f) } else { rng.pick(&n.m) }.clone();
    // a share of people carry a name from another language group (migration, travel)
    let last = if rng.chance(0.12) {
        let langs: Vec<&String> = names.keys().collect();
        let lang = *rng.pick(&langs);
        rng.pick(&names[lang].last).clone()
    } else {
        rng.pick(&n.last).clone()
    };
    Person { first, last }
}

pub struct EmailDomains {
    by_country: std::collections::BTreeMap<String, (Vec<String>, Weighted)>,
}

impl EmailDomains {
    pub fn new() -> EmailDomains {
        let mut by_country = std::collections::BTreeMap::new();
        for (c, doms) in &world().people.email_domains {
            let names: Vec<String> = doms.keys().cloned().collect();
            let w = Weighted::new(doms.values().copied());
            by_country.insert(c.clone(), (names, w));
        }
        EmailDomains { by_country }
    }

    pub fn pick(&self, rng: &mut Rng, iso2: &str) -> String {
        let (names, w) = self.by_country.get(iso2).unwrap_or_else(|| &self.by_country["*"]);
        names[w.sample(rng)].clone()
    }
}

/// Email local part in one of the common styles; `n` disambiguates (0 = none).
pub fn email_local(rng: &mut Rng, p: &Person, n: u32) -> String {
    let f = ascii_fold(&p.first).replace(' ', "");
    let l = ascii_fold(&p.last).replace(' ', "");
    let style = rng.below(10);
    let base = match style {
        0..=3 => format!("{f}.{l}"),
        4 => format!("{f}{l}"),
        5 => format!("{}.{l}", &f[..1.min(f.len())]),
        6 => format!("{f}_{l}"),
        7 => format!("{f}{}", &l[..1.min(l.len())]),
        8 => format!("{l}.{f}"),
        _ => format!("{f}.{l}"),
    };
    if n == 0 { base } else { format!("{base}{n}") }
}

pub fn phone(rng: &mut Rng, iso2: &str) -> String {
    let f = &world().people.phone_formats;
    let pat = f.get(iso2).unwrap_or(&f["*"]);
    pat.chars().map(|c| if c == '#' { (b'0' + rng.below(10) as u8) as char } else { c }).collect()
}

pub fn postal(rng: &mut Rng, pattern: &str) -> String {
    let alts: Vec<&str> = pattern.split('|').collect();
    let p = rng.pick(&alts);
    const L: &[u8] = b"ABDEFGHJLNPQRSTUWXYZ";
    p.chars()
        .map(|c| match c {
            '#' => (b'0' + rng.below(10) as u8) as char,
            '@' => L[rng.below(L.len())] as char,
            c => c,
        })
        .collect()
}

#[derive(Clone)]
pub struct Address {
    pub country: usize,
    pub region: usize,
    #[allow(dead_code)] // part of the data model, not used by the simulator yet
    pub city: usize,
    pub line1: String,
    pub line2: Option<String>,
    pub city_name: String,
    pub postal: Option<String>,
    pub lat: f64,
    pub lon: f64,
    /// coordinates known (addresses imported from older systems were never geocoded)
    pub geocoded: bool,
}

/// Data typed by people (or migrated from older systems) is messy: inconsistent casing,
/// extra spaces, unnormalised postal codes and phone numbers. `mess` is the probability of
/// each kind of mistake (merchants differ in data hygiene).
pub fn dirty_name(rng: &mut Rng, name: &str, mess: f64) -> String {
    if !rng.chance(mess) {
        return name.to_string();
    }
    match rng.below(5) {
        0 | 1 => name.to_lowercase(),
        2 => name.to_uppercase(),
        3 => name.replacen(' ', "  ", 1),
        _ => format!("{name} "),
    }
}

pub fn dirty_address(rng: &mut Rng, a: &mut Address, mess: f64) {
    if rng.chance(mess) {
        a.city_name = if rng.chance(0.7) { a.city_name.to_lowercase() } else { a.city_name.to_uppercase() };
    }
    if rng.chance(mess) {
        if let Some(p) = a.postal.as_mut() {
            *p = if rng.chance(0.6) { p.replace(' ', "") } else { p.to_lowercase() };
        }
    }
    if rng.chance(mess * 0.5) {
        a.line1 = if rng.chance(0.5) { format!("{},", a.line1) } else { a.line1.replace("Street", "St.").replace("straße", "str.").replace("Straße", "Str.") };
    }
    if a.line2.is_none() && rng.chance(mess * 0.6) {
        a.line2 = Some(String::new());
    }
}

/// The same phone number written another way (spaces, national prefix, dashes).
pub fn phone_variant(rng: &mut Rng, phone: &str, calling_code: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let cc: String = calling_code.chars().filter(|c| c.is_ascii_digit()).collect();
    let national = digits.strip_prefix(&cc).unwrap_or(&digits).to_string();
    match rng.below(4) {
        0 => format!("0{national}"),
        1 => format!("00{cc}{national}"),
        2 => format!("+{cc}{national}"),
        _ => format!("({}) {}-{}", &national[..3.min(national.len())], &national[3.min(national.len())..6.min(national.len())], &national[6.min(national.len())..]),
    }
}

pub fn address(rng: &mut Rng, country_idx: usize) -> Address {
    let w = world();
    let c = &w.geo.countries[country_idx];
    // bigger regions/cities come first in the data: weight by position
    let regions = Weighted::new((0..c.regions.len()).map(|i| 1.0 / (1.0 + i as f64 * 0.6)));
    let ri = regions.sample(rng);
    let reg = &c.regions[ri];
    let cities = Weighted::new((0..reg.cities.len()).map(|i| 1.0 / (1.0 + i as f64)));
    let ci = cities.sample(rng);
    let city = &reg.cities[ci];
    let streets = w.geo.streets.get(&c.language).unwrap_or(&w.geo.streets["en"]);
    let street = rng.pick(streets);
    let num = 1 + (rng.exp(25.0) as u32).min(400);
    let line1 = if c.address_style == "street_num" { format!("{street} {num}") } else { format!("{num} {street}") };
    let line2 = if rng.chance(0.18) {
        Some(match c.language.as_str() {
            "de" => format!("{}. OG", 1 + rng.below(5)),
            "nl" => format!("{}", ["A", "B", "-1", "-2", "hs"][rng.below(5)]),
            "fr" => format!("Appartement {}", 1 + rng.below(40)),
            _ => {
                if c.iso2 == "US" { format!("Apt {}{}", 1 + rng.below(20), ["A", "B", "C", ""][rng.below(4)]) } else { format!("Flat {}", 1 + rng.below(30)) }
            }
        })
    } else {
        None
    };
    Address {
        country: country_idx,
        region: ri,
        city: ci,
        line1,
        line2,
        city_name: city.name.clone(),
        postal: if city.postal.is_empty() { None } else { Some(postal(rng, &city.postal)) },
        lat: city.lat + rng.gauss(0.0, 0.03),
        lon: city.lon + rng.gauss(0.0, 0.05),
        geocoded: true,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Desktop,
    Mobile,
    Tablet,
}

pub fn device(rng: &mut Rng, mobile_share: f64) -> Device {
    let x = rng.f64();
    if x < mobile_share { Device::Mobile } else if x < mobile_share + 0.07 { Device::Tablet } else { Device::Desktop }
}

pub fn user_agent(rng: &mut Rng, d: Device) -> &'static str {
    const DESKTOP: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
    ];
    const MOBILE: &[&str] = &[
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36",
        "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/120.0.6099.119 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 14; SM-A546B) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/25.0 Chrome/121.0.0.0 Mobile Safari/537.36",
    ];
    const TABLET: &[&str] = &[
        "Mozilla/5.0 (iPad; CPU OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 13; SM-X710) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
    ];
    match d {
        Device::Desktop => rng.pick(DESKTOP),
        Device::Mobile => rng.pick(MOBILE),
        Device::Tablet => rng.pick(TABLET),
    }
}

/// A public-looking IPv4 address (avoids private and reserved ranges), sometimes IPv6.
pub fn ip(rng: &mut Rng) -> String {
    if rng.chance(0.12) {
        return format!("2a02:{:x}:{:x}:{:x}::{:x}", rng.int(0x100, 0xffff), rng.int(0, 0xffff), rng.int(0, 0xffff), rng.int(1, 0xffff));
    }
    loop {
        let a = rng.int(2, 223);
        if [10, 100, 127, 169, 172, 192, 198, 203].contains(&a) {
            continue;
        }
        return format!("{a}.{}.{}.{}", rng.int(0, 255), rng.int(0, 255), rng.int(1, 254));
    }
}

/// bcrypt-looking hash string (not a real hash of anything).
pub fn password_hash(rng: &mut Rng) -> String {
    const A: &[u8] = b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let body: String = (0..53).map(|_| A[rng.below(A.len())] as char).collect();
    format!("$2b$12${body}")
}
