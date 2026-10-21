# Tidewick Commerce: company briefing

**Tidewick Commerce** is an all-in-one commerce platform for mid-size merchants. It provides:

- storefront and checkout;
- payments;
- inventory and fulfillment;
- subscriptions;
- a light B2B CRM;
- a helpdesk;
- marketing tools.

Tidewick was founded in 2018 in Amsterdam. It has offices in London and Austin and about 400 employees.

## Merchants

About 120 merchants run their business on Tidewick, and each merchant is a tenant of the platform. A handful of large merchants generate most of the volume, with a long tail of small ones behind them.

The merchants work in these verticals:

- apparel and footwear;
- consumer electronics;
- beauty;
- home and living;
- sports and outdoor;
- food and coffee subscriptions;
- pet supplies;
- a few B2B wholesalers (industrial supplies, office equipment).

Merchants are mostly based in the UK, the Netherlands, Germany, France, the Nordics and the US. Their shoppers come from about 40 countries.

## How Tidewick makes money

Merchants pay a monthly platform plan. Tidewick also keeps a processing fee on every payment it settles for them; payouts show this as gross, fee and net.

## The data

- Production runs on one multi-tenant database: every business table carries the merchant's `tenant_id`.
- The production database is replicated nightly into the analytics warehouse.
- Tidewick moved to this database at the start of 2023, so history starts on 2023-01-01.
- Timestamps are stored in UTC.
- Amounts are in the merchant's currency. `exchange_rates` has daily rates from EUR.
- The full table catalog is in `schema.md`.

## Today

Today is **Monday, 2026-01-05**. The warehouse holds complete data through 2025-12-31.

Tidewick has just hired several new leaders. Each of them wants to understand their part of the business quickly and relies on the analytics team to do so.

The analytics team works through the warehouse, as described in `handbook.md`.
