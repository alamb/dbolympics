# Simulator design

The world it simulates is described in [WORLD-MODEL.md](WORLD-MODEL.md).

## Goals

- **Deterministic**: the same `(scale factor, seed)` produces identical Parquet files on any
  machine and with any number of threads.
- **Fast and parallel**: SF100 in minutes on a laptop.
- **Coherent**: every FK resolves, money adds up, timestamps follow causes, tenants are
  isolated; all of it validated, not assumed.
- **Realistic physical order**: like a production database, ids grow with time and rows are
  written roughly in insertion order, with merchants interleaved by day.

## Time-major simulation

The simulator advances one day at a time over 2023-01-01..2025-12-31 (day 0 also carries the
rows that existed before the 2023 migration, with their original `created_at`). Each day:

1. **Merchant phase** (parallel over merchants): merchant-level processes, e.g. joining,
   staff, catalog launches and archiving, warehouses. Writes the merchant's shared state.
2. **Shopper phase** (parallel over shopper chunks): sessions, carts, orders and everything
   downstream. Reads merchant state (read-only), writes only its own chunk state.
3. **Finalize** (parallel over tables): assign ids, resolve references, write Parquet.

When something starts, its near future is decided right away (an order's payment,
fulfillment, shipment, tracking, delivery, return...), and those rows are queued for the days
they happen. So a row is complete when written, and the database is a snapshot as of
2025-12-31: anything planned after that is cut off, and statuses reflect that.

## Chunks

A chunk is a unit of work with its own RNG streams and row buffers:
chunk 0 is global reference data, then for each merchant one merchant chunk and k shopper
chunks (k grows with the merchant's expected shoppers). The chunk layout depends only on the
scale factor, never on the thread count. New shoppers are routed to a chunk by a hash of
their email address (local part as generated, and domain; B2B buyers by company), so all
possible email duplicates meet in one chunk and are made unique there.

Shared merchant aggregates (stock levels, coupon redemption counts, ...) are not mutated by
shopper chunks directly: chunks record commutative effects (add / max) that are merged after
the day, which keeps the result independent of scheduling.

## Ids and references

- Every table with a bigint `id` gets dense ids `1..n`. Ids are assigned at finalize in the
  order (day, chunk, insertion order), so ids grow with time across merchants.
- While a day is open, a new row is referenced by a pending handle (table, day, chunk,
  index); it becomes `base[table][day] + offset[table][day][chunk] + index` when the day is
  finalized. Rows may only reference rows of the same or earlier days.
- Per-tenant sequences (order numbers, invoice numbers) are assigned at finalize, in id order.
- Polymorphic references (audit `entity_id`, stock movement `reference_id`, ...) resolve like
  foreign keys; tables referenced only that way are registered with `set_poly_targets` so
  their id offsets are kept.
- A row can take its id without being written (`Row::skip`), for rows deleted before the
  snapshot (archived audit rows): ids then have gaps like after a real purge.

## Streaming vs deferred tables

- **Streaming** tables (facts: orders, events, shipments, ...) are written when their day is
  finalized.
- **Deferred** tables hold rows whose columns describe end state (users.last_login_at,
  tenants.status, loyalty balances, stock quantities, ...). They get their id on the day they
  are created, stay in memory, can be updated, and are written at the end.

## Fundamentals: behaviour emerges from a few mechanisms

Instead of hand-coding effects or tuning distributions, the simulator gives entities latent
traits and applies five mechanisms; variety, correlations, anticorrelations and skew follow
from them. When a value looks unrealistic, the fix is to model what sets it in reality.

1. **Latent traits.** Products: appeal (visibility), quality, price relative to their type,
   fit bias and fit uncertainty, fragility, season. Shoppers: taste (product types), own size,
   budget (country income level x personal), price and quality sensitivity, patience,
   consumption cycle. Carriers, warehouses and support desks: capacity.
2. **Choice by utility.** List pages pick the most attractive of a few candidates; add-to-cart
   follows the product's utility (quality, price vs budget, season, taste); baskets favour
   complementary product types (`complements` in world/catalog.json). Emerges: units fall
   with price, seasonal product types, items bought together, bigger baskets for richer
   shoppers, multiples of cheap items, baskets topped up to the free-shipping threshold.
3. **Expectation vs experience.** Each order pre-rolls lateness vs the promised date, realised
   vs price-implied quality, fit, damage, picking errors, remorse. Emerges: returns and their
   reasons by vertical, bracketing (two sizes, one returned), J-shaped review ratings that track
   product quality, tickets, satisfaction and churn.
4. **Capacity and queues** (src/sim/capacity.rs). Daily capacity sized to the trailing 8 weeks plus
   headroom and peak-season staffing; yesterday's volume joins a backlog that delays today's
   work. Emerges: peak-season delivery delays and "where is my order" waves, slow support under
   load. Stories change capacities (src/sim/stories.rs).
5. **Needs and routines.** Consumables run out and are rebought (mostly the same products);
   gifts in the holiday season (other sizes, other addresses, the January return wave); the
   device depends on the time of day (mobile on commutes and evenings, desktop in office hours).

## Customer journeys and web analytics

Every visit is a simulated path: landing page by acquisition channel (paid search and social
land on product pages, email on collections, direct on the home page), then category pages,
search, product pages, recommendation clicks (`product_relations`), add-to-cart, cart and
checkout. The probabilities depend on the visit's intent (browse / research / buy), channel,
device and each product's attractiveness (view-to-cart rate); what is added to the cart is what
gets bought. Carts can be left and bought on a later visit. Visitors often browse anonymously
before signing up; the signup keeps their `anonymous_id`, and shoppers also use a second
device with its own anonymous id.

Web analytics is **sampled**: only 15% of consented sessions are stored (`web_sessions`,
`page_views`, `events`), and each event's JSON carries `"sample_rate":0.15`. Orders, carts and
all server-side data are complete. Comparing analytics with orders therefore needs scaling, as
in real analytics setups. Unsampled journeys are still simulated (they decide carts and
orders) but produce no strings.

## Marketing

Merchants send newsletters (weekly to monthly by size), promotion announcements and win-back
mails to segments (all subscribers, VIPs, lapsed customers); 15% of sends test two subject
lines. Shoppers opt in at signup (fewer under GDPR) and have a latent email engagement.
Engaged subscribers open and click; a click is an email-channel visit to the campaign's
landing page with its `utm_campaign`, so email traffic in web analytics, `utm_links` clicks,
`landing_pages` views and email-attributed orders all come from the same clicks. Every email
risks an unsubscribe, more so for disengaged readers, so lists decay; hard bounces suppress an
address. Apple Mail Privacy Protection "opens" every mail for some iPhone users (opens without
clicks). Paid orders get a last-click attribution (an email clicked within 7 days, else the
visit's channel; paid channels map to the merchant's always-on campaigns) and first orders a
first-touch attribution. Loyalty points are credited 30 days after delivery on what was not
returned; happy customers create referral codes that friends in the same shopper chunk use.
Consent records follow the same facts: cookie consent decides whether sessions are tracked.

## Subscriptions

Subscription merchants (coffee clubs, pet food autoship, beauty and letterbox boxes) offer
2-4 plans. Shoppers start a subscription instead of a one-off checkout, more often when a
consumable runs out. Each billing period is an invoice with a payment (no order); a failed
charge enters dunning (retries after 3, 5 and 7 days) and ends the subscription when all
fail. Each delivery is an order with a 100% "prepaid" discount that is fulfilled like any
other, so late or damaged boxes lower satisfaction. At every renewal the subscriber decides
to stay or cancel based on satisfaction, a price shock after a price change (scaled by
their price sensitivity) and whether they still shop here; cancellations happen a few days
before the renewal. Price changes create new plan versions (new plan and plan price rows);
subscribers move to them at their next renewal. Annual plans are billed once and ship
monthly. Extra bags are metered (`usage_records`) and billed with the next invoice.

## Back office, identity and privacy

Merchants have roles with permissions, teams, invited staff who sign in every few working
days, API keys and service accounts for their integrations, settings, feature flags, jobs on
cron schedules, integration syncs, imports and retention policies. Staff actions (sign-ins,
product launches, price changes, campaigns, exports, return approvals, GDPR work) go to the
audit log; price and status changes also to `change_history`. The monthly archival job moves
audit rows older than 24 months to `audit_log_archive`; they keep their ids, so `audit_log`
has a gap at the start. Job runs are kept 90 days and health checks 14 days, as their
retention policies say. Shopper sign-ins create sessions (30 days per device), with failed
attempts, password resets and social sign-in. EU/UK shoppers sometimes file GDPR requests;
an erased shopper is anonymised in `users` and their orders lose `customer_email`, while
other logs keep the address.

## B2B

Wholesale buyers belong to companies: the merchant routes buyers of one company to one
shopper chunk, where the first buyer opens the account (often from a lead) and later buyers
join as further contacts. A new account goes through a sales opportunity to a yearly
framework agreement (deal); six weeks before it ends the account renews with a probability
that rises with its buyers' satisfaction, so bad deliveries and slow support lose contracts,
and most buyers of a lost account stop ordering. Accounts on payment terms (net 14/30/60) get
an invoice when the order ships and pay by bank transfer around the due date (some habitually
late, with reminders; a few never); returns become credit notes, deducted from the payment
or kept as store credit. Large orders are quoted first (draft orders); purchase-order numbers
appear as order notes.

## Analytics, gift cards

Nightly jobs roll up yesterday into `metrics_daily` (from the stored, sampled sessions and
events, so they inherit sampling and any event bugs) and monthly KPIs into `kpi_snapshots`;
monthly signup cohorts are materialised on the 1st. The largest merchants run storefront
experiments (assignment on first exposure; the treatment changes conversion a little).
Feature usage is counted per shopper and day. Gift cards are sold as gift presents (no VAT,
booked as a liability), handed to another shopper who redeems them weeks later (Christmas
cards in January), partly or never (breakage); refunds of orders paid with a gift card go to
store credit (`wallets`).

## Drivers instead of fixed outcomes

Two examples of values set by their drivers rather than by a distribution.

### Paid media (`campaigns` with channel `paid_search` / `paid_social`)

Paid media runs as quarterly flights per channel, each planned two weeks before its quarter;
a merchant's first flights cover the rest of its joining quarter, marked "(launch)". A flight
still ahead on the last day of data is `scheduled`, one cut short by churn `paused`. A
flight's budget is set by:

- **Revenue:** the merchant's trailing 91 days (a rough estimate in its first month), scaled
  by the expected demand of the quarter ahead relative to the last 91 days, so seasonal plans
  emerge. Consumer brands add an October push.
- **Hidden marketing intensity:** paid spend as a share of revenue, by vertical times a
  log-normal. Beauty and apparel spend most, B2B about 1%.
- **Hidden channel split:** the social share by vertical (beauty and apparel lean social,
  electronics and B2B search) plus noise.
- **Feedback:** each channel's budget scale moves with observed return on ad spend (ROAS)
  against the merchant's target of 1.3 / gross margin times noise. Search is judged by
  last-click revenue, social (which finds new customers) by last click plus first touch. A
  scale that sinks to its floor writes the channel off.
- **Human rounding** to a tenth, quarter or half of the order of magnitude, by habit; naming
  and UTM conventions also differ per merchant.

Each day the spend sets the merchant's channel mix: paid weight = base × ((spend / revenue) /
reference / click-price index)^0.6, capped at 3 times the base; clicks are dearer in Q4 and by
6% a year. Paid spend adds new customers on top of unpaid demand (`paid_volume`); established
merchants get more organic and direct traffic. Returning visitors click ads in proportion to
the merchant's search spend (less so B2B buyers); people first found through an ad mostly
return through organic search or directly. Without a running flight a visit cannot be a paid
click. At SF1 the paid share of sessions ranges from 0% to 27% between merchants and
correlates 0.88 with spend relative to revenue.

### Supplier deliveries (`purchase_orders`)

The delivery date depends on origin (domestic, European, Asian: longer and more variable lead
times the farther away), a hidden supplier reliability (a bias, how optimistic the quoted lead
time is, and a spread; some suppliers are punctual, some chronically late), order size (larger
than usual for that supplier takes a little longer) and seasonal congestion (the peak season
October to early December; Chinese New Year for Asian suppliers, adding 8-16 days; August
shutdowns for European suppliers).

`expected_at` uses the merchant's planning lead time, which also drives the reorder point.
Merchants with tight operations, more often larger ones, update it from actual deliveries
(exponential smoothing); the others keep the quoted time. So lateness shrinks over time at
learning merchants, and a sudden slowdown (S4) shows as a spike that fades only where the
merchant learns. Goods arrive during working hours; the goods receipt has the PO's
`received_at` time.

## Data hygiene

Each merchant has a hidden data-hygiene level. Customer data typed into its shop is
sometimes messy: names in lower or upper case or with extra spaces, cities in odd casing,
postal codes without spaces, empty strings instead of NULL, phone numbers in other formats.
Customers imported from a previous platform are messier still and less complete (no locale
or time zone, no coordinates). Emails are always generated in lower case (they stand in for
case-insensitive text).

## Output

`<out>/<table>/<period>.parquet`, zstd: every table gets one file per period, even if empty.
The 5 periods are `2024-and-earlier`, then the quarters `2025-Q1` ... `2025-Q4`; the names
sort in time order.

- **`2024-and-earlier`** holds the rows that existed before the 2023 migration (the engine
  creates them on day 0, together with the activity of 2023-01-01) and the rows the engine
  created in 2023 and 2024.
- **A quarter file** holds the rows the engine created on the days of that quarter of 2025, in id
  order. Rows of deferred tables go by their creation day too, with their final values.
- **Rows imported later** keep their arrival quarter: a merchant that joins in 2025 brings
  products with an older `created_at`, and they are in that quarter's file.
- **Prefixes:** rows only reference rows of the same or an earlier period, so the files up to
  any quarter are referentially complete (`dbolympics check-periods`). Deferred rows with final
  values are a known exception, reported and not an error.

Types: bigint/integer/smallint -> Int64/Int32/Int16, decimal -> Decimal128,
timestamp -> Timestamp(us, no tz), date -> Date32, varchar/uuid/json -> Utf8,
`varchar ARRAY` -> List<Utf8>.

At write time every row is checked against the schema: NOT NULL, varchar lengths, integer
ranges, decimal precision.

## Randomness

An own xoshiro256** generator seeded per (seed, chunk, day, purpose) and distributions built
on `libm` (not the platform libm), so results do not depend on the OS or on crate versions.

## Static world data

`world/*.json`: merchants, geography, people (names and email domains per country) and catalog
vocabulary per vertical (category trees, product types, variant attributes, price ranges);
sources in [DATA-SOURCES.md](DATA-SOURCES.md). Computed: `calendar_days` (one row per day
2023-01-01..2026-12-31, the same at every SF) and `exchange_rates` (a daily EUR rate, a random
walk between the levels in `world/geo.json` moving on business days; weekends repeat Friday's
rate, as forward-filled warehouse rate tables do), both in `src/sim/reference.rs`.

## Checking a change

1. `cargo build --release`, then `dbolympics generate --sf 1 --out data/sf1`.
2. `dbolympics validate data/sf1`: loads everything into DuckDB with all primary keys,
   foreign keys, uniques and checks, then runs the rule queries of `schema/ddl/rules.sql`
   (they must return no rows). Self-referencing foreign keys are loaded level by level.
3. Determinism: generate with `--threads 1` and `--threads 8` and compare the SHA-256 of all
   Parquet files. They must be identical; this also covers row order.
4. Plausibility and story queries in DuckDB: ledger groups balance, stock equals the sum of
   movements, coupon counts match, the story signals are visible.
5. Before a release also SF10 (`validate` takes about 45 s): uniqueness problems across
   shopper chunks only show up at larger scale factors.
