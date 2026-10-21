# Data sources and licensing

Everything the generator uses, where it comes from, and why it may be used in a benchmark.
Every input below permits commercial use, so the generated data is not encumbered.

**No real personal data is used.** All people, companies, products and events in the generated
data are synthetic.

## Software and schema

| Input | Source | License | Use |
|---|---|---|---|
| Base schema | heron, commit 859f7c3 | Apache-2.0 (`schema/LICENSE-heron`, `schema/NOTICE-heron`) | Imported and converted to `schema/schema.json`; attribution kept |
| SQL reserved words (`tools/reserved_words.txt`) | PostgreSQL 18 documentation, Appendix C (SQL:2023 and PostgreSQL columns) | PostgreSQL License (permissive); a keyword list is factual | Avoiding reserved identifiers |
| DuckDB reserved keywords | `duckdb_keywords()` of DuckDB 1.5 | MIT | Same |
| Rust crates | see `Cargo.lock` (DuckDB, Arrow, Parquet, chrono, libm, rayon, clap, serde, anyhow) | MIT and/or Apache-2.0 | Build-time and run-time dependencies; not part of the generated data |

## World data (`world/`)

Factual reference data (country codes, tax rates, coordinates, common names) is not
copyrightable; it was compiled by hand from general knowledge and public sources, not copied
from any database, and spot-checked against the sources listed.

| File / field | Content | Source and status |
|---|---|---|
| `geo.json` countries | ISO 3166-1 alpha-2/alpha-3 codes, names, calling codes, continents, currencies | ISO 3166 / ITU-T E.164 codes are public identifiers, free to use as data |
| `geo.json` VAT / GST rates | Standard rates with the changes inside 2023-2025 (e.g. Finland 24% -> 25.5% on 2024-09-01, Estonia 20% -> 22% -> 24%, Romania 19% -> 21% on 2025-08-01, Slovakia 20% -> 23% on 2025-01-01, Switzerland 7.7% -> 8.1%, Luxembourg's temporary 16% in 2023, Singapore 8% -> 9%) | Public law. Checked 2026-10-02 against public summaries of the European Commission's rate tables, e.g. https://www.vatai.com/blog/2025-vat-rates-in-europe-country-rates-changes . Only standard rates are modelled; reduced rates are not |
| `geo.json` `us_sales_tax` | Approximate combined (state + average local) sales tax per US state | Approximations at the level published by e.g. the Tax Foundation; not exact |
| `geo.json` regions and cities | Region names, a few major cities per region with approximate coordinates (3 decimals) | Facts from general knowledge. Region codes are simplified and **not** authoritative ISO 3166-2 codes |
| `geo.json` postal patterns | Postal code formats per city (e.g. Berlin `1####`, Amsterdam `10## @@`, UK outward codes) | Formats are facts; the generated codes are random within the format and may or may not exist |
| `geo.json` streets | Common street names per language ("High Street", "Hauptstraße", "Kerkstraat", ...) | Generic, extremely common names |
| `geo.json` `income_index` | Relative income level per country (UK = 1.0), used for shopper budgets | Rounded, approximate ratios of GDP per capita at PPP as published by the IMF / World Bank; only the rough ordering matters |
| `geo.json` currencies, FX | Currency codes (ISO 4217), symbols, minor units; EUR rates at the start and end of the window | ISO 4217 codes are public. The rates are rough levels in line with ECB euro reference rates; the daily series is **synthetic** (a random walk between the two levels, moving on business days, weekends forward-filled), not ECB data |
| `geo.json` locales | BCP 47 locale tags with English and native names | Public identifiers |
| Marketing behaviour | Opt-in, open, click and unsubscribe rates in the range of public email-marketing benchmarks; Apple Mail Privacy Protection (since iOS 15) pre-fetching emails; analytics cookie consent valid for 13 months as recommended by the French CNIL | Own modelling assumptions based on general knowledge; no data copied |
| `catalog.json` seasons, complements | Which product types are winter / summer items, which are bought together | Own modelling assumptions |
| `people.json` names | Common first and last names per language | Generic, very common names from general knowledge; combined at random, so generated people are fictional |
| `people.json` email domains | Shares of common mailbox providers per country (gmail.com, web.de, gmx.de, orange.fr, ziggo.nl, ...) | Approximate market shares from general knowledge. Real domain names make the data look real; see "Synthetic personal data" below |
| `people.json` phone formats | Mobile number formats per country | Formats only; numbers are random |
| `merchants.json`, carriers, payment gateways; `catalog.json` brands and products | The platform, its ~120 merchants, carriers, gateways and catalog | **Fictional.** Names that play a role in the stories are checked against real companies (below) |

## Fictional names

The platform (Tidewick Commerce), merchants, carriers, payment gateways, suppliers, brands and
products are invented. Every name used in a story (platform, story merchants, carriers,
gateways) was checked in 2026 with a web search (name plus industry), in the UK Companies House
register and in TMview (EU trade marks and national offices including EUIPO, DPMA, UKIPO, BOIP
and the USPTO). Names that collided with a real business or mark were replaced, and the
replacements checked the same way. Near matches kept (a different name, field or country):
Moravell, Halyard Pay, Kennet Freight, Gallia Colis.

Background merchants and the product brands in `catalog.json` are not individually checked and
may coincide with real shops or brands by accident; any resemblance is coincidental. No story
attributes a negative event to them: stories only involve the checked names and the
merchants' own-brand products.

## Fictional systems, domains and URLs

The platform's own infrastructure is fictional and named generically: mail relays `twmail-eu-1`
/ `twmail-us-1`, SMS gateway `twsms-1`, analytics library `tidewick-analytics.js` (mobile:
`tidewick-mobile-sdk`), message ids under `mail.tidewick.app`, address validation service
`qgeo-1`, secret stores `platform_vault` / `merchant_kms`, export downloads under
`exports.tidewick.app`, integration connectors named by kind (`erp_connector`, `wms_connector`,
...). Social sign-in providers appear only as the generic provider names `google` and `apple`.
Real third-party SaaS providers (email, SMS, analytics, payments) are not named. Merchant
websites, CDN and webhook URLs are built from the fictional merchant names (e.g.
`https://erp.marrowbyandvane.co.uk/hooks/tidewick`), carrier tracking URLs from the fictional
carrier names. Such a domain may happen to exist; the URLs are data, are never fetched, and
imply nothing about whoever owns the domain.

## Synthetic personal data

Generated people have random combinations of common names, random addresses within real postal
formats, random phone numbers and email addresses at real providers. Such an address may by
chance exist; the data must never be used to contact anyone. GDPR does not apply to the
generated data because it relates to no real person, but this is why the dataset is labelled
synthetic everywhere it is published.

## License of the generated data

MIT, like the rest of the repository (`LICENSE`). Nothing above restricts it: all inputs are
either facts, permissively licensed (Apache-2.0, MIT, PostgreSQL License), or our own.
