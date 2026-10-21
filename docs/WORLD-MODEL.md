# World model

What the simulator simulates. Section 1 is also what the query-generation agents are told
([QUERY-GENERATION.md](QUERY-GENERATION.md)); the rest of this file and all of
[STORIES.md](STORIES.md) stay hidden from them.

## 1. The company (agent-facing backstory)

**Tidewick Commerce** is an all-in-one commerce platform for mid-size merchants: storefront,
checkout and payments, inventory and fulfillment, subscriptions, a light B2B CRM, a helpdesk,
and marketing tools, all in one product. Founded 2018 in Amsterdam, offices in London and
Austin, ~400 employees.

- **Customers ("merchants")**: ~120 merchants run their business on Tidewick. Each merchant is
  a tenant of the platform. A handful of large merchants generate most of the volume; there
  is a long tail of small ones. Verticals: apparel and footwear, consumer electronics, beauty,
  home and living, sports and outdoor, food and coffee subscriptions, pet supplies, and a few
  B2B wholesalers (industrial supplies, office equipment).
- **How Tidewick makes money**: merchants pay a monthly platform plan, and Tidewick keeps a
  processing fee on every payment it settles for them (visible in payouts: gross, fee, net).
- **Where**: merchants are mostly in the UK, the Netherlands, Germany, France, the Nordics
  and the US; their shoppers come from ~40 countries.
- **The data**: the production database is one multi-tenant database (every business table
  carries the merchant's tenant_id). It is replicated nightly into the analytics warehouse.
  Tidewick migrated to this database at the start of 2023, so history starts 2023-01-01.
- **Today** is Monday, 2026-01-05. Data is complete through 2025-12-31.
- **The situation**: Tidewick has just hired several new leaders (see QUERY-GENERATION.md, "The
  organisation and how it talks"). They want to understand the business, and they rely on the
  analytics team for it.

## 2. Entities and roles

| Entity | Rows | Notes |
|---|---|---|
| Merchant | `tenants` (+ settings, integrations, webhooks, api keys, feature-flag overrides) | Fixed set of ~120 at every scale factor. Lifecycle: trial -> active -> (past_due) -> churned. |
| Shopper | `users` with `is_staff = false` | Belongs to one merchant. Has addresses, devices, sessions, carts, orders, reviews, tickets, loyalty, consents. |
| Merchant staff | `users` with `is_staff = true` | Admins, support agents (`support_agents`), sales reps (CRM owners), warehouse staff (receipts, adjustments), marketers (campaigns). Roles via `user_roles`/`teams`. Some staff also shop at their own merchant (messy, realistic). |
| B2B buyer | `users` + `crm.accounts`/`contacts` + `billing_profiles` | Only at wholesale merchants. Quotes (`draft_orders`), invoices with net terms. |
| Product | `products`, `product_variants`, prices, media, categories, brands | Per merchant. Popularity is Zipf-skewed and has a lifecycle (launch, peak, decline, archive). |
| Supplier / warehouse | `suppliers`, `warehouses`, `stock_*`, `purchase_orders` | Per merchant; lead times and reliability vary by supplier. |
| Campaign | `campaigns`, `segments`, `ab_tests`, `utm_links`, `promotions`, `coupons` | Drive sessions, signups and orders. |

There is no separate "customer" table: shoppers, staff and B2B buyers are all `users`, told
apart by `is_staff`, roles and which tables reference them. The ambiguity is intentional.

The plan fees have no table: the platform's revenue is only visible as processing fees
(`payouts.fee_amount`).

## 3. Hidden traits (drive behaviour, never stored directly)

- **Merchant**: vertical, home country and currency, size (Zipf), growth curve, join date,
  plan, payment gateway mix, carriers, support staffing, marketing intensity, data hygiene.
- **Shopper**: country, acquisition channel (organic, paid search, social, email, referral,
  influencer coupon), device mix, purchase propensity, price sensitivity, return propensity,
  patience (tickets, churn), lifetime.
- **Product**: category, price tier, margin, return rate, seasonality, popularity lifecycle.

## 4. Processes -> tables

1. **Merchant lifecycle**: signup, trial, activation, plan, staff accounts, roles, teams,
   integrations and syncs, webhooks, API keys, settings, feature flags, eventual churn.
   (`identity.*`, `ops.*`, `comms.webhooks`)
2. **Catalog**: products, variants, media, categories, tags, collections, bundles, price
   lists and price changes over time (`catalog.*`, `pricing.prices`).
3. **Inventory**: suppliers, purchase orders, inbound receipts, stock movements,
   reservations, adjustments, stockouts (`inventory.*`).
4. **Shopper journey**: acquisition -> web session (page views, events with nested JSON
   properties, device profile) -> cart -> order -> payment (attempts, failures, retries) ->
   fulfillment -> shipment and tracking -> delivery -> review / return / refund / ticket ->
   loyalty -> repeat purchase or churn (`analytics.*`, `sales.*`, `billing.payments*`,
   `catalog.product_reviews`, `support.*`, `marketing.loyalty_*`).
5. **Subscriptions** (coffee, pet food, beauty boxes): subscriptions, invoices, usage,
   dunning, cancellations (`billing.subscriptions`, `invoices`, `dunning_attempts`, ...).
6. **B2B**: leads, opportunities, deals, quotes, net-terms invoices, credit notes
   (`crm.*`, `sales.draft_orders`, `billing.invoices`, `credit_notes`).
7. **Money**: every payment, refund and fee produces double-entry ledger lines; daily
   payouts net of Tidewick's fee (`billing.ledger_*`, `payouts`, `payout_items`).
8. **Marketing**: segments, campaigns, sends and engagement, A/B tests, attributions,
   referrals, unsubscribes (`marketing.*`, `comms.email_log`, `comms.notifications`).
9. **Support**: tickets driven by real causes (late delivery, defect, payment failure,
   account issue), messages, events, SLA breaches, CSAT (`support.*`).
10. **Ops and compliance**: audit log of staff actions, change history, access log, job
    runs, imports and import errors, health checks, GDPR requests and deletions
    (`audit.*`, `ops.*`).

Correlations come from causes, not from sampling: a late shipment raises the chance of a
ticket, a bad ticket lowers CSAT, low CSAT lowers repeat purchases, and so on.

## 5. Time

- Window: 2023-01-01 to 2025-12-31, all timestamps UTC.
- Daily / weekly / seasonal patterns in each merchant's local time zone; Black Friday,
  Cyber Monday, Christmas, January sales, summer slump; vertical-specific seasonality.
- Merchants join over time (the platform grows), so early months have fewer merchants.

## 6. Scale factors

- **SF = GB of uncompressed CSV-equivalent data** (SF1 ~ 1 GB); the Parquet (zstd) output is
  several times smaller. The design target is SF1-SF100; larger SFs must work but need not
  stay realistic.
- **Fixed at every SF**: the ~120 merchants (ids, names, verticals, countries, relative
  sizes), the time window, geography, lookup tables, and all stories (same merchants,
  dates, carriers, gateways, products, effect sizes). Every literal a query uses exists at
  every SF, and per-merchant selectivity stays constant (like TPC-H's fixed nations).
- **Grows with sqrt(SF)**: catalog size, staff, warehouses, suppliers, campaigns, segments,
  integrations: a merchant with 2,500 orders/day has a bigger catalog and team than one
  with 25.
- **Linear in SF**: shoppers and everything they do (sessions, events, carts, orders and
  downstream). Proportions (conversion, refund rate, ticket rate) do not change.
- Merchant sizes are Zipf-shaped: the largest merchant has ~19% of the volume. Orders per
  day for the whole platform at SF1: ~47 on average over 2023 to 2025, ~60 in autumn 2025
  and ~100 in December 2025; linear in SF (SF100: ~4,700 on average).
- About 0.94 GB CSV-equivalent per SF unit plus 0.17 GB that does not scale (catalogs, staff
  activity, rollups). SF1 has ~12k shoppers, ~52k orders, ~95k stored (sampled) web sessions
  and ~380k events; web analytics is ~29% of the bytes.
- Product-level stories (one product, one lot) are faint at SF1 and clear from SF10.
