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


# Analytics handbook

This handbook explains how the analytics team at Tidewick turns questions into numbers.

## The people

- **Leaders** (CEO, CFO, department heads) bring broad questions about their area.
- **Analysts** each work for one leader. An analyst:
  - turns the leader's questions into concrete report requests;
  - reads the printed reports and follows up;
  - writes a short memo for the leader at the end of the week.
- **Data engineers** turn each report request into one SQL query. They can check the query against the schema as often as they like before submitting it.

## The warehouse is expensive

The warehouse is shared and metered, and every executed query is billed to the department. We therefore run reports as an overnight batch:

- An analyst has **3 report slots per night** and the week has **4 nights**.
- The memo is due on Friday morning.
- A slot is spent whenever a request is submitted, whether it runs, fails or is bounced by review.
- Unused slots do not carry over.
- Checking a query against the schema (a "dry run") is free. It runs the query against an empty copy of the warehouse schema, so it catches errors and shows the output columns, but it never reads or shows any data.

Think before you request. Make one request answer one question well.

## One request, one report

Each request produces one printed report:

- **Title and question.** Every report has a title and the analyst's question it answers.
- **One grain.** A report is one table at a single grain: every row is the same kind of thing (a country, a week, a merchant, a carrier and month). The columns are measures or attributes of that thing.
- **The page has a width.** A report has at most 12 columns, and about 10 is comfortable.
- **The page has a length.** At most 50 rows are printed. The report notes how many rows the query returned in total.
- **Readable columns.** Order the rows so the important ones come first, and give columns readable names.

Dense reports are good. For example, one row per country and month can carry these columns:

- orders;
- revenue;
- refund rate;
- share of late deliveries;
- tickets per 100 orders;
- the same month last year.

Data engineering reviews every request before it runs and **bounces bundled ones**, where several unrelated questions are packed into one report. Reports wider than 12 columns are bounced too. A bounced request still uses its slot.

If you have two questions, request two reports.

## House SQL standards (for data engineers)

- Write **one SELECT statement**. Do not end it with a semicolon.
- Use **standard SQL** wherever possible.
- **Dates:** never use `CURRENT_DATE` or `now()`. The reporting date is 2026-01-05 and the data ends on 2025-12-31.
- **Deterministic ordering:** always finish with an ORDER BY whose tie-breaks make the order unique.
- **Tenants:** filter or group by `tenant_id` when a question is about particular merchants. Join `tenants` for merchant names.
- **Money:** amounts are in the currency given next to them (usually a `currency_code` column; a merchant's main currency is `tenants.default_currency_code`).
  - Convert amounts before adding them up across merchants that use different currencies.
  - `exchange_rates` holds a rate from EUR (`base_code`) for every day; weekends carry the previous business day's rate.
- **Rates:** round rates and averages for the page.
- **Catalog:** the table catalog (`schema.md`) lists every table, column, foreign key and the allowed codes of status columns.


# Table catalog

# Tidewick Commerce data warehouse: table catalog

The production database, replicated nightly. Every business table carries the merchant's
`tenant_id`. Timestamps are UTC. Money columns are in the row's `currency_code` (merchants sell
in their own currency); `exchange_rates` has daily EUR reference rates.
Status columns reference small lookup tables; their codes are listed with each table.
All tables live in one schema: refer to them by name only (`shipments`, not `sales.shipments`).
The sections below only group the tables by business area.

## Area: analytics

### web_sessions
Visitor analytics sessions, distinct from sessions (auth). Anonymous traffic still produces a session with a null user_id.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `anonymous_id` uuid (Client-generated visitor identifier used to stitch sessions before the user authenticates.)
- `started_at` timestamp not null
- `ended_at` timestamp
- `landing_page` varchar(2048)
- `referrer` varchar(2048)
- `utm_source` varchar(255)
- `utm_medium` varchar(255)
- `utm_campaign` varchar(255)
- `ip_address` varchar(45)
- `user_agent` varchar(2048)
- `is_bounce` boolean not null
- `page_view_count` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### events
The primary product analytics fact table. Each row is one tracked event with a free-form event_name and schemaless JSON properties.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `web_session_id` bigint -> web_sessions (Nullable link to the analytics web session this event occurred in; server-side events may have none.)
- `event_name` varchar(64) not null
- `properties` json not null
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### page_views
Individual page hits within a web session, used for path and dwell-time analysis.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `web_session_id` bigint not null -> web_sessions
- `url` varchar(2048) not null
- `path` varchar(2048)
- `title` varchar(255)
- `referrer` varchar(2048)
- `duration_seconds` integer
- `viewed_at` timestamp not null
- `created_at` timestamp not null

### device_profiles
Browser/device fingerprints observed per tenant, optionally linked to a user once authenticated.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `fingerprint` varchar(255) not null
- `device_type` varchar(64)
- `os` varchar(64)
- `os_version` varchar(64)
- `browser` varchar(64)
- `browser_version` varchar(64)
- `screen_width` integer
- `screen_height` integer
- `first_seen_at` timestamp not null
- `last_seen_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, fingerprint)

### feature_usage
Per-user, per-day product feature adoption counters feeding engagement and stickiness metrics.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `feature_key` varchar(64) not null (Stable identifier of the product feature being measured (e.g. bulk_export, dashboard_share).)
- `usage_count` integer not null
- `usage_day` date not null
- `last_used_at` timestamp
- `occurred_at` timestamp not null
- `created_at` timestamp not null
- unique: (tenant_id, user_id, feature_key, usage_day)

### experiments
Server-side product experiments (feature experiments), distinct from ab_tests which are message-level tests.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `key` varchar(64) not null
- `name` varchar(255) not null
- `description` varchar(4000)
- `hypothesis` varchar(4000)
- `status` varchar(32) not null one of: draft, running, paused, completed, archived
- `primary_metric` varchar(255)
- `starts_at` timestamp
- `ends_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, key)

### experiment_variants
Variants (arms) of an experiment; exactly one may be flagged is_control, with allocation_pct summing across arms.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `experiment_id` bigint not null -> experiments
- `key` varchar(64) not null
- `name` varchar(255) not null
- `is_control` boolean not null
- `allocation_pct` decimal(5,2) not null
- `config` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (experiment_id, key)

### experiment_assignments
Records which variant a user (or anonymous visitor) was bucketed into for an experiment.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `experiment_id` bigint not null -> experiments
- `variant_id` bigint not null -> experiment_variants
- `user_id` bigint -> users
- `anonymous_id` uuid
- `assigned_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (experiment_id, user_id)

### funnels
Ordered conversion-funnel definitions evaluated over the events stream.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `is_active` boolean not null
- `window_days` integer
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### funnel_steps
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `funnel_id` bigint not null -> funnels
- `step_order` integer not null
- `name` varchar(255) not null
- `event_name` varchar(64) not null
- `filters` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (funnel_id, step_order)

### cohorts
Named user cohorts, dynamic (rule-based) or static, used for retention and segmentation analysis.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `definition` json not null
- `is_dynamic` boolean not null
- `member_count` integer not null
- `computed_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### cohort_members
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `cohort_id` bigint not null -> cohorts
- `user_id` bigint not null -> users
- `added_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (cohort_id, user_id)

### metrics_daily
Pre-aggregated daily metric values per tenant; one row per (tenant, metric_key, day) for fast dashboard reads.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `metric_key` varchar(64) not null
- `metric_date` date not null
- `metric_value` decimal(20,6) not null
- `dimensions` json not null (Optional breakdown labels (e.g. {"channel":"organic"}) when a metric is sliced; empty object for the headline total.)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, metric_key, metric_date)

### kpi_snapshots
Periodic captures of headline KPIs (MRR, active users, conversion rate) with the prior-period value for trend deltas.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `kpi_key` varchar(64) not null
- `period_grain` varchar(64) not null
- `period_start` date not null
- `period_end` date not null
- `kpi_value` decimal(20,6) not null
- `previous_value` decimal(20,6)
- `metadata` json not null
- `captured_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, kpi_key, period_grain, period_start)

## Area: audit

### audit_log
Central append-only audit trail of meaningful mutations and security actions. The audited row is referenced polymorphically via entity_schema/entity_table/entity_id (no FK).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `action` varchar(32) not null one of: create, update, delete, login, logout, export, access_denied
- `actor_user_id` bigint -> users
- `actor_type` varchar(64) not null (Principal kind that performed the action: user, api_key, service_account, or system.)
- `entity_schema` varchar(64)
- `entity_table` varchar(64)
- `entity_id` bigint (Primary-key id of the audited row in entity_schema.entity_table; intentionally not a foreign key.)
- `summary` varchar(4000)
- `metadata` json not null
- `ip_address` varchar(45)
- `user_agent` varchar(2048)
- `request_id` varchar(64)
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### change_history
Column-level before/after JSON diffs for an audited row, usually linked to an audit_log entry.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `audit_log_id` bigint -> audit_log
- `actor_user_id` bigint -> users
- `entity_schema` varchar(64) not null
- `entity_table` varchar(64) not null
- `entity_id` bigint not null
- `changed_columns` varchar(255)[] not null
- `before_data` json not null
- `after_data` json not null
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### access_log
Append-only record of reads of sensitive/PII data, for compliance and breach forensics.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `actor_user_id` bigint -> users
- `entity_schema` varchar(64) not null
- `entity_table` varchar(64) not null
- `entity_id` bigint
- `access_kind` varchar(64) not null
- `field_names` varchar(255)[] not null
- `purpose` varchar(64)
- `ip_address` varchar(45)
- `user_agent` varchar(2048)
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### data_exports
data_exports — GDPR / data-portability export jobs. Tracks request through completion; the produced artifact lives in object storage (file_url).
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `requested_by_user_id` bigint -> users
- `subject_user_id` bigint -> users
- `status` varchar(64) not null
- `format` varchar(64) not null
- `file_url` varchar(2048)
- `row_count` bigint
- `requested_at` timestamp not null
- `completed_at` timestamp
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id)

### consent_records
Append-only per-user consent grants and withdrawals by purpose; the latest row per (user, purpose) is the effective consent state.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `purpose` varchar(64) not null
- `status` varchar(32) not null one of: granted, withdrawn, expired
- `source` varchar(64)
- `policy_version` varchar(64) (Version of the privacy policy / terms the user consented to at the time.)
- `ip_address` varchar(45)
- `granted_at` timestamp
- `withdrawn_at` timestamp
- `expires_at` timestamp
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### compliance_events
compliance_events — flagged compliance incidents / policy violations needing review (suspicious access, retention overrun, failed DSAR SLA, ...).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `event_type` varchar(64) not null
- `severity` varchar(64) not null
- `detected_by_user_id` bigint -> users
- `entity_schema` varchar(64)
- `entity_table` varchar(64)
- `entity_id` bigint
- `details` json not null
- `resolved` boolean not null
- `resolved_at` timestamp
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### data_subject_requests
GDPR/CCPA data-subject requests (access, erasure, rectification, portability) tracked through their processing lifecycle.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `subject_user_id` bigint -> users
- `subject_email` varchar(254)
- `request_type` varchar(64) not null (Kind of DSAR: access, erasure, rectification, portability, or restriction.)
- `status` varchar(64) not null
- `assigned_to_user_id` bigint -> users
- `data_export_id` bigint -> data_exports
- `notes` varchar(4000)
- `received_at` timestamp not null
- `due_at` timestamp
- `completed_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id)

### retention_policies
Per-entity data-retention rules (retention_days + action_on_expiry) driving automated purges.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `entity_schema` varchar(64) not null
- `entity_table` varchar(64) not null
- `retention_days` integer not null
- `action_on_expiry` varchar(64) not null
- `is_active` boolean not null
- `last_run_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, entity_schema, entity_table)

### legal_holds
Active legal/investigation holds that suspend retention-driven deletion for a subject or entity scope.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `reason` varchar(4000)
- `retention_policy_id` bigint -> retention_policies
- `subject_user_id` bigint -> users
- `entity_schema` varchar(64)
- `entity_table` varchar(64)
- `entity_id` bigint
- `placed_by_user_id` bigint -> users
- `is_active` boolean not null
- `placed_at` timestamp not null
- `released_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### audit_log_archive
Cold archive of aged-out audit_log rows; same shape as audit_log, written once by the archival job. source_audit_log_id preserves the original id.
- `id` bigint not null
- `source_audit_log_id` bigint
- `tenant_id` bigint not null -> tenants
- `action` varchar(32) not null one of: create, update, delete, login, logout, export, access_denied
- `actor_user_id` bigint
- `actor_type` varchar(64) not null
- `entity_schema` varchar(64)
- `entity_table` varchar(64)
- `entity_id` bigint
- `summary` varchar(4000)
- `metadata` json not null
- `ip_address` varchar(45)
- `user_agent` varchar(2048)
- `request_id` varchar(64)
- `occurred_at` timestamp not null
- `archived_at` timestamp not null
- `created_at` timestamp not null

## Area: billing

### billing_profiles
Customer billing entity per tenant: legal name, tax id, billing email, net payment terms. One marked is_default per tenant.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `legal_name` varchar(255) not null
- `billing_email` varchar(254)
- `tax_id` varchar(64)
- `net_terms_days` integer not null (Payment terms in days (Net-0/Net-30/Net-60) before an invoice is past due.)
- `default_currency_code` varchar(3) not null -> currencies
- `is_default` boolean not null
- `metadata` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp

### payment_methods
Stored payment instruments. Only a gateway token plus brand/last4 are kept; raw card numbers are never stored.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `billing_profile_id` bigint -> billing_profiles
- `kind` varchar(32) not null one of: card, bank_transfer, paypal, wallet, wire, credit
- `gateway` varchar(64) not null
- `gateway_token` varchar(255) not null
- `brand` varchar(255)
- `last4` varchar(4)
- `exp_month` smallint
- `exp_year` smallint
- `is_default` boolean not null
- `billing_address_id` bigint -> addresses
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (gateway, gateway_token)

### payments
Money-in captures against an order and/or invoice. amount_refunded tracks cumulative refunds for partial-refund accounting.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `order_id` bigint -> orders
- `invoice_id` bigint -> invoices
- `payment_method_id` bigint -> payment_methods
- `status` varchar(32) not null one of: pending, authorized, captured, failed, voided, refunded, partially_refunded
- `amount` decimal(14,4) not null
- `amount_refunded` decimal(14,4) not null (Running total of amounts refunded against this payment; must never exceed amount.)
- `currency_code` varchar(3) not null -> currencies
- `gateway` varchar(64) not null
- `gateway_charge_id` varchar(255)
- `authorized_at` timestamp
- `captured_at` timestamp
- `failure_code` varchar(64)
- `metadata` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### payment_attempts
Each gateway round-trip for a payment; a payment may need several attempts before it captures.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `payment_id` bigint not null -> payments
- `attempt_number` integer not null
- `status` varchar(32) not null one of: pending, authorized, captured, failed, voided, refunded, partially_refunded
- `amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `gateway_response` json not null
- `error_code` varchar(64)
- `error_message` varchar(4000)
- `attempted_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (payment_id, attempt_number)

### payment_refunds
Refunds reversing all or part of a captured payment, tracked through the refund_status lifecycle.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `payment_id` bigint not null -> payments
- `status` varchar(32) not null one of: pending, succeeded, failed, cancelled
- `amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `reason` varchar(4000)
- `gateway_refund_id` varchar(255)
- `processed_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### invoices
Billing documents (one-off or subscription). amount_due drives dunning; public_id is the opaque external identifier.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `billing_profile_id` bigint -> billing_profiles
- `user_id` bigint -> users
- `order_id` bigint -> orders
- `subscription_id` bigint -> subscriptions
- `invoice_number` varchar(64) not null
- `status` varchar(32) not null one of: draft, open, paid, void, uncollectible, past_due
- `currency_code` varchar(3) not null -> currencies
- `subtotal` decimal(14,4) not null
- `tax_total` decimal(14,4) not null
- `discount_total` decimal(14,4) not null
- `total` decimal(14,4) not null
- `amount_paid` decimal(14,4) not null
- `amount_due` decimal(14,4) not null (Outstanding balance still owed; reaches 0 when the invoice is fully paid.)
- `issued_at` timestamp
- `due_at` timestamp
- `paid_at` timestamp
- `voided_at` timestamp
- `notes` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id); (tenant_id, invoice_number)

### invoice_lines
Line items on an invoice; optionally tied to a subscription_item, with the service period it covers.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `invoice_id` bigint not null -> invoices
- `subscription_item_id` bigint -> subscription_items
- `description` varchar(4000) not null
- `quantity` decimal(14,4) not null
- `unit_price` decimal(14,4) not null
- `line_total` decimal(14,4) not null
- `tax_amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `period_start` timestamp
- `period_end` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### credit_notes
Credit memos issued against invoices for returns, goodwill, or corrections.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `invoice_id` bigint -> invoices
- `billing_profile_id` bigint -> billing_profiles
- `credit_note_number` varchar(64) not null
- `currency_code` varchar(3) not null -> currencies
- `subtotal` decimal(14,4) not null
- `tax_total` decimal(14,4) not null
- `total` decimal(14,4) not null
- `reason` varchar(4000)
- `issued_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, credit_note_number)

### credit_note_lines
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `credit_note_id` bigint not null -> credit_notes
- `invoice_line_id` bigint -> invoice_lines
- `description` varchar(4000) not null
- `quantity` decimal(14,4) not null
- `unit_price` decimal(14,4) not null
- `line_total` decimal(14,4) not null
- `tax_amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null

### payouts
Settlements of collected funds out to the tenant; net = gross minus fees.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `currency_code` varchar(3) not null -> currencies
- `gross_amount` decimal(14,4) not null
- `fee_amount` decimal(14,4) not null
- `net_amount` decimal(14,4) not null
- `status` varchar(64) not null
- `gateway_payout_id` varchar(255)
- `arrival_date` date
- `paid_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### payout_items
The individual payments settled within a payout; each payment appears in at most one payout.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `payout_id` bigint not null -> payouts
- `payment_id` bigint not null -> payments
- `amount` decimal(14,4) not null
- `fee_amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (payment_id)

### ledger_accounts
ledger_accounts — the tenant's chart of accounts for the double-entry ledger. type is the accounting class (asset/liability/revenue/...).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `code` varchar(64) not null
- `name` varchar(255) not null
- `account_type` varchar(64) not null
- `currency_code` varchar(3) not null -> currencies
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, code)

### ledger_entries
Immutable append-only double-entry ledger lines. Postings sharing an entry_group form one balanced journal entry.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ledger_account_id` bigint not null -> ledger_accounts
- `entry_group` uuid not null (Groups the debit/credit rows of a single balanced journal entry (their amounts must net to zero).)
- `direction` varchar(32) not null one of: debit, credit
- `amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `payment_id` bigint -> payments
- `invoice_id` bigint -> invoices
- `description` varchar(4000)
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### plans
Subscription products. code is the stable per-tenant key; default_interval sets the billing cadence.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `code` varchar(64) not null
- `name` varchar(255) not null
- `description` varchar(4000)
- `default_interval` varchar(32) not null one of: day, week, month, quarter, year
- `interval_count` integer not null
- `trial_period_days` integer not null
- `is_active` boolean not null
- `is_public` boolean not null
- `metadata` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, code)

### plan_features
Feature entitlements/limits per plan; limit_value NULL means unlimited.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `plan_id` bigint not null -> plans
- `feature_key` varchar(64) not null
- `limit_value` bigint (Quota for the feature (e.g. seats, api_calls); NULL means unlimited.)
- `is_enabled` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (plan_id, feature_key)

### plan_prices
Price points for a plan, one per currency and billing interval.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `plan_id` bigint not null -> plans
- `currency_code` varchar(3) not null -> currencies
- `billing_interval` varchar(32) not null one of: day, week, month, quarter, year
- `interval_count` integer not null
- `amount` decimal(14,4) not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (plan_id, currency_code, billing_interval, interval_count)

### subscriptions
A user's recurring commitment to a plan. current_period_* bound the next invoice; cancel_at_period_end defers cancellation.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `plan_id` bigint not null -> plans
- `billing_profile_id` bigint -> billing_profiles
- `payment_method_id` bigint -> payment_methods
- `status` varchar(32) not null one of: trialing, active, past_due, canceled, paused, incomplete
- `currency_code` varchar(3) not null -> currencies
- `quantity` integer not null
- `trial_ends_at` timestamp
- `current_period_start` timestamp
- `current_period_end` timestamp
- `cancel_at_period_end` boolean not null (When true the subscription cancels at current_period_end rather than immediately.)
- `canceled_at` timestamp
- `started_at` timestamp not null
- `ended_at` timestamp
- `metadata` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id)

### subscription_items
Line items / add-ons of a subscription; unit_amount snapshots the price at the time of subscription.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `subscription_id` bigint not null -> subscriptions
- `plan_price_id` bigint -> plan_prices
- `description` varchar(4000)
- `quantity` integer not null
- `unit_amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `is_metered` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### usage_records
Metered usage reported for a subscription_item, aggregated into consumption-based invoice lines.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `subscription_item_id` bigint not null -> subscription_items
- `quantity` decimal(14,4) not null
- `unit` varchar(64)
- `action` varchar(64) not null
- `occurred_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### dunning_attempts
Retry sequence chasing payment on a past-due invoice; next_attempt_at schedules the following retry.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `invoice_id` bigint not null -> invoices
- `subscription_id` bigint -> subscriptions
- `payment_id` bigint -> payments
- `attempt_number` integer not null
- `outcome` varchar(64) not null
- `next_attempt_at` timestamp
- `attempted_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (invoice_id, attempt_number)

### tax_registrations
Tenant tax IDs per jurisdiction (VAT/GST/sales tax) governing how tax is charged in a region.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `billing_profile_id` bigint -> billing_profiles
- `country_code` varchar(2) not null
- `jurisdiction` varchar(64)
- `tax_type` varchar(64) not null
- `registration_number` varchar(64) not null (The official tax/VAT registration number issued by the jurisdiction.)
- `is_active` boolean not null
- `valid_from` date
- `valid_until` date
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, country_code, tax_type)

### wallets
Stored-credit balances per user and currency (prepaid credit, refunds-to-credit, promo balance).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `currency_code` varchar(3) not null -> currencies
- `balance` decimal(14,4) not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (user_id, currency_code)

## Area: catalog

### brands
Manufacturer/brand records that products are grouped under.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `description` varchar(4000)
- `logo_url` varchar(2048)
- `website_url` varchar(2048)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, slug)

### categories
Hierarchical product taxonomy; parent_id is a self-reference to the parent category.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `parent_id` bigint -> categories
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `description` varchar(4000)
- `sort_position` integer not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, slug)

### category_closure
Transitive closure of the category tree (ancestor, descendant, depth) for fast subtree queries.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ancestor_id` bigint not null -> categories
- `descendant_id` bigint not null -> categories
- `depth` integer not null (Number of edges between ancestor and descendant; 0 is the self-row.)
- `created_at` timestamp not null
- unique: (ancestor_id, descendant_id)

### products
Product master records; public_id is the opaque external identifier exposed via API/URL.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `brand_id` bigint -> brands
- `primary_category_id` bigint -> categories
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `description` varchar(4000)
- `status` varchar(32) not null one of: draft, active, archived
- `is_visible` boolean not null
- `published_at` timestamp
- `seo_title` varchar(255)
- `seo_description` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (public_id); (tenant_id, slug)

### product_variants
Sellable SKUs of a product; sku is unique per tenant. Reference price only — authoritative pricing lives in the pricing schema.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `product_id` bigint not null -> products
- `sku` varchar(64) not null
- `barcode` varchar(64)
- `name` varchar(255)
- `sort_position` integer not null
- `price_amount` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `weight_grams` decimal(14,4)
- `is_default` boolean not null (Exactly one variant per product may be the default (enforced by a partial unique index).)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (public_id); (tenant_id, sku)

### product_categories
product_categories — junction products×categories (a product can sit in many categories beyond its primary one).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `product_id` bigint not null -> products
- `category_id` bigint not null -> categories
- `sort_position` integer not null
- `created_at` timestamp not null
- unique: (product_id, category_id)

### attributes
Definitions of product/variant attributes (e.g. Color, Size); kind drives value interpretation.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `code` varchar(64) not null
- `name` varchar(255) not null
- `kind` varchar(32) not null one of: text, number, boolean, enum, color, dimension
- `is_variant_defining` boolean not null (True when this attribute differentiates variants (Color, Size) rather than being descriptive.)
- `is_filterable` boolean not null
- `unit` varchar(64)
- `sort_position` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, code)

### attribute_values
Allowed values for enum-kind attributes.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `attribute_id` bigint not null -> attributes
- `value_code` varchar(255) not null
- `label` varchar(255)
- `sort_position` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (attribute_id, value_code)

### variant_attribute_values
Junction binding a variant to its attribute/value pairs (the combination that makes it unique).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `variant_id` bigint not null -> product_variants
- `attribute_id` bigint not null -> attributes
- `attribute_value_id` bigint -> attribute_values
- `value_text` varchar(4000)
- `created_at` timestamp not null
- unique: (variant_id, attribute_id)

### product_media
Images, video, and documents attached to a product and optionally a specific variant.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `product_id` bigint not null -> products
- `variant_id` bigint -> product_variants
- `kind` varchar(32) not null one of: image, video, document, model_3d
- `url` varchar(2048) not null
- `alt_text` varchar(4000)
- `sort_position` integer not null
- `is_primary` boolean not null
- `width_px` integer
- `height_px` integer
- `created_at` timestamp not null
- `updated_at` timestamp not null

### collections
Merchandising groupings of products; may be manual or rule-driven (is_automated).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `description` varchar(4000)
- `is_automated` boolean not null
- `rules` json
- `starts_at` timestamp
- `ends_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, slug)

### collection_products
collection_products — junction collections×products.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `collection_id` bigint not null -> collections
- `product_id` bigint not null -> products
- `sort_position` integer not null
- `created_at` timestamp not null
- unique: (collection_id, product_id)

### tags
tags — free-form labels applied to products.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, slug)

### product_tags
product_tags — junction products×tags.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `product_id` bigint not null -> products
- `tag_id` bigint not null -> tags
- `created_at` timestamp not null
- unique: (product_id, tag_id)

### product_reviews
Customer reviews of products, rated 1-5 stars; one review per user per product.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `product_id` bigint not null -> products
- `user_id` bigint -> users
- `rating` integer not null
- `title` varchar(255)
- `body` varchar(65535)
- `is_verified_purchase` boolean not null (True when the reviewer is known to have purchased the product.)
- `is_approved` boolean not null
- `helpful_count` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (product_id, user_id)

### review_votes
review_votes — helpfulness votes cast by users on a review.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `review_id` bigint not null -> product_reviews
- `user_id` bigint not null -> users
- `is_helpful` boolean not null
- `created_at` timestamp not null
- unique: (review_id, user_id)

### product_relations
Directional cross-sell/upsell/related links between two products.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `product_id` bigint not null -> products
- `related_product_id` bigint not null -> products
- `relation_kind` varchar(64) not null
- `sort_position` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (product_id, related_product_id, relation_kind)

### product_bundles
Header for a bundle/kit sold as a single offering, composed of variant components.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `product_id` bigint -> products
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `description` varchar(4000)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, slug)

### bundle_items
Component variants and quantities that make up a product bundle.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `bundle_id` bigint not null -> product_bundles
- `variant_id` bigint not null -> product_variants
- `quantity` integer not null
- `sort_position` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (bundle_id, variant_id)

## Area: comms

### notification_templates
Reusable, per-tenant notification bodies keyed by code and channel; rendered into notifications and email/sms logs.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `code` varchar(64) not null
- `name` varchar(255) not null
- `channel` varchar(64) not null
- `subject` varchar(255)
- `body` varchar(65535) not null
- `locale` varchar(5)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, code)

### notifications
Per-user notifications with a queued→sent→delivered→read lifecycle. template_id is null when composed ad hoc.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `template_id` bigint -> notification_templates
- `channel` varchar(64) not null
- `status` varchar(32) not null one of: queued, sent, delivered, read, failed
- `title` varchar(255) not null
- `body` varchar(65535)
- `data` json not null
- `sent_at` timestamp
- `delivered_at` timestamp
- `read_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### notification_preferences
Per-user opt-in matrix across channels and categories; an absent row means the tenant default applies.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `channel` varchar(64) not null
- `category` varchar(255) not null
- `is_enabled` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (user_id, channel, category)

### message_threads
In-app conversation threads between a tenant and a user. last_message_at is denormalized for inbox sorting.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `subject` varchar(255)
- `channel` varchar(64) not null
- `is_closed` boolean not null
- `last_message_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### thread_messages
thread_messages — individual messages within a thread. direction marks whether the message is inbound (from the user) or outbound (from the tenant).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `thread_id` bigint not null -> message_threads
- `sender_user_id` bigint -> users
- `direction` varchar(32) not null one of: inbound, outbound (inbound = sent by the user toward the tenant; outbound = sent by the tenant toward the user.)
- `body` varchar(65535) not null
- `attachments` json not null
- `read_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### webhooks
Subscriber HTTP endpoints that receive signed event callbacks. Only the hash of the signing secret is stored.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `created_by_user_id` bigint -> users
- `name` varchar(255) not null
- `target_url` varchar(2048) not null
- `secret_hash` varchar(255) not null (Hash of the HMAC signing secret used to sign delivery payloads; the plaintext secret is shown once at creation.)
- `event_types` varchar(64)[] not null
- `is_active` boolean not null
- `last_delivered_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### webhook_deliveries
Append-only log of each webhook delivery attempt, including HTTP response_code and retry attempt number.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `webhook_id` bigint not null -> webhooks
- `event_type` varchar(64) not null
- `status` varchar(32) not null one of: pending, success, failed, retrying
- `attempt` integer not null (1-based retry counter; multiple rows per logical event when a delivery is retried.)
- `response_code` integer
- `request_body` json
- `response_body` varchar(65535)
- `duration_ms` integer
- `delivered_at` timestamp
- `created_at` timestamp not null

### email_log
Immutable record of every outbound email; recipient_email is denormalized since the user or address may be off-platform.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `template_id` bigint -> notification_templates
- `recipient_email` varchar(254) not null
- `from_email` varchar(254)
- `subject` varchar(255)
- `provider` varchar(64)
- `provider_message_id` varchar(255)
- `status` varchar(32) not null one of: pending, success, failed, retrying
- `error_message` varchar(4000)
- `sent_at` timestamp
- `created_at` timestamp not null

### sms_log
sms_log — IMMUTABLE record of each outbound SMS. (Intentionally uncommented at table level — see CONVENTIONS.md on deliberate comment gaps.)
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `template_id` bigint -> notification_templates
- `recipient_phone` varchar(32) not null
- `from_number` varchar(32)
- `body` varchar(65535) not null
- `provider` varchar(64)
- `provider_message_id` varchar(255)
- `status` varchar(32) not null one of: pending, success, failed, retrying
- `error_message` varchar(4000)
- `segments` integer not null
- `sent_at` timestamp
- `created_at` timestamp not null

### push_tokens
Device push-notification tokens per user; deactivated rather than deleted when a device unregisters.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `platform` varchar(64) not null
- `token` varchar(255) not null
- `device_name` varchar(255)
- `is_active` boolean not null
- `last_used_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, token)

### unsubscribes
Suppression list of addresses/channels that must be excluded from further sends; user_id may be null for off-platform addresses.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `channel` varchar(64) not null
- `address` varchar(255) not null
- `category` varchar(255) not null
- `reason` varchar(4000)
- `unsubscribed_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, channel, address, category)

### contact_channels
Verified contact endpoints (email/phone) per user with a verification lifecycle; one primary per kind via a partial unique index.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `kind` varchar(64) not null
- `channel_address` varchar(255) not null
- `is_primary` boolean not null
- `is_verified` boolean not null
- `verified_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (user_id, kind, channel_address)

## Area: crm

### accounts
The tenant's own B2B customer companies. Tenant-scoped; distinct from tenants (the platform's customers).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `legal_name` varchar(255)
- `domain` varchar(255)
- `industry` varchar(255)
- `employee_count` integer
- `annual_revenue` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `owner_user_id` bigint -> users (Internal sales rep (users) who owns the account relationship.)
- `phone` varchar(32)
- `website` varchar(2048)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### contacts
People who work at a CRM account. primary_email is denormalized; the full set lives in contact_emails.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `account_id` bigint -> accounts
- `owner_user_id` bigint -> users
- `first_name` varchar(255)
- `last_name` varchar(255)
- `primary_email` varchar(254)
- `phone` varchar(32)
- `job_title` varchar(255)
- `department` varchar(255)
- `is_primary` boolean not null
- `do_not_contact` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp

### contact_emails
All email addresses for a contact; at most one flagged is_primary per contact.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `contact_id` bigint not null -> contacts
- `email` varchar(254) not null
- `label` varchar(255) not null
- `is_primary` boolean not null
- `verified_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (contact_id, email)

### leads
Unqualified inbound interest before it becomes an account/contact; converts forward once qualified.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `owner_user_id` bigint -> users
- `first_name` varchar(255)
- `last_name` varchar(255)
- `email` varchar(254)
- `phone` varchar(32)
- `company` varchar(255)
- `job_title` varchar(255)
- `status` varchar(32) not null one of: new, contacted, qualified, unqualified, converted
- `source` varchar(64)
- `score` integer not null (Lead score (0+); higher means more sales-ready.)
- `converted_account_id` bigint -> accounts
- `converted_contact_id` bigint -> contacts
- `converted_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### pipelines
Named sales pipelines per tenant; at most one is the default. Stages live in opportunity_stages.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `is_default` boolean not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### opportunity_stages
Ordered lookup of stages within a pipeline. position sets funnel order; win_probability drives weighted forecasts.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `pipeline_id` bigint not null -> pipelines
- `name` varchar(255) not null
- `sort_position` integer not null
- `win_probability` decimal(5,4) not null (Default close probability for opportunities in this stage, 0..1.)
- `is_won` boolean not null
- `is_lost` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (pipeline_id, sort_position); (pipeline_id, name)

### opportunities
Deals in flight: an account being worked through a pipeline stage with an expected amount.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `account_id` bigint not null -> accounts
- `pipeline_id` bigint not null -> pipelines
- `stage_id` bigint not null -> opportunity_stages
- `primary_contact_id` bigint -> contacts
- `owner_user_id` bigint -> users
- `name` varchar(255) not null
- `amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `probability` decimal(5,4)
- `source` varchar(64)
- `is_closed` boolean not null
- `is_won` boolean not null
- `expected_close_date` date
- `closed_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### deals
Closed-won contracts realized from an opportunity, holding the booked contract value and term.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `account_id` bigint not null -> accounts
- `opportunity_id` bigint -> opportunities
- `owner_user_id` bigint -> users
- `name` varchar(255) not null
- `contract_value` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `signed_at` date
- `starts_at` date
- `ends_at` date
- `created_at` timestamp not null
- `updated_at` timestamp not null

### deal_line_items
Products sold on a deal, priced per unit in the deal currency.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `deal_id` bigint not null -> deals
- `product_id` bigint -> products
- `description` varchar(4000)
- `quantity` integer not null
- `unit_price` decimal(14,4) not null
- `discount_pct` decimal(5,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null

### activities
Logged interactions (calls, emails, meetings) against an account, contact, or opportunity.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `kind` varchar(32) not null one of: call, email, meeting, note, task
- `account_id` bigint -> accounts
- `contact_id` bigint -> contacts
- `opportunity_id` bigint -> opportunities
- `owner_user_id` bigint -> users
- `subject` varchar(255)
- `body` varchar(65535)
- `occurred_at` timestamp not null
- `duration_minutes` integer
- `created_at` timestamp not null
- `updated_at` timestamp not null

### crm_notes
crm_notes — free-text notes attached to a CRM entity (polymorphic target).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `account_id` bigint -> accounts
- `contact_id` bigint -> contacts
- `opportunity_id` bigint -> opportunities
- `author_user_id` bigint -> users
- `body` varchar(65535) not null
- `is_pinned` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### tasks
Follow-up to-dos assigned to reps, optionally linked to a CRM entity.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `assignee_user_id` bigint -> users
- `account_id` bigint -> accounts
- `contact_id` bigint -> contacts
- `opportunity_id` bigint -> opportunities
- `title` varchar(255) not null
- `description` varchar(4000)
- `priority` varchar(64) not null
- `is_done` boolean not null
- `due_at` timestamp
- `completed_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### account_relationships
Parent/subsidiary and other directed links between accounts within a tenant.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `parent_account_id` bigint not null -> accounts
- `child_account_id` bigint not null -> accounts
- `relationship_type` varchar(64) not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (parent_account_id, child_account_id, relationship_type)

### contact_lists
contact_lists — static, manually-curated lists of contacts.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `owner_user_id` bigint -> users
- `name` varchar(255) not null
- `description` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### contact_list_members
contact_list_members — junction of contacts into static lists.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `contact_list_id` bigint not null -> contact_lists
- `contact_id` bigint not null -> contacts
- `added_by_user_id` bigint -> users
- `created_at` timestamp not null
- unique: (contact_list_id, contact_id)

## Area: geo

### countries
ISO-3166 country reference. Global, shared by all tenants.
- `id` bigint not null
- `iso2` varchar(2) not null
- `iso3` varchar(3) not null
- `name` varchar(255) not null
- `phone_code` varchar(32)
- `continent` varchar(64)
- unique: (iso2); (iso3)

### regions
- `id` bigint not null
- `country_id` bigint not null -> countries
- `code` varchar(64)
- `name` varchar(255) not null

### cities
- `id` bigint not null
- `region_id` bigint -> regions
- `country_id` bigint not null -> countries
- `name` varchar(255) not null
- `latitude` decimal(9,6)
- `longitude` decimal(9,6)

### currencies
ISO-4217 currencies. The code column (e.g. USD) is the FK target used by all monetary tables.
- `id` bigint not null
- `code` varchar(3) not null
- `name` varchar(255) not null
- `symbol` varchar(64)
- `minor_unit` smallint not null (Number of decimal places (2 for USD, 0 for JPY).)
- unique: (code)

### exchange_rates
Daily FX rates between currency pairs, used to normalize multi-currency revenue.
- `id` bigint not null
- `base_code` varchar(3) not null -> currencies
- `quote_code` varchar(3) not null -> currencies
- `rate` decimal(12,6) not null
- `as_of` date not null
- unique: (base_code, quote_code, as_of)

### calendar_days
Calendar dimension: one row per day from 2023-01-01 to 2026-12-31 with week, month and quarter attributes. Join to it to build complete time series (days, weeks or months without activity still appear).
- `calendar_date` date not null (The day.)
- `year_num` integer not null (Calendar year.)
- `quarter_num` smallint not null (Calendar quarter, 1-4.)
- `month_num` smallint not null (Calendar month, 1-12.)
- `month_name` varchar(9) not null (English month name, e.g. 'November'.)
- `month_label` varchar(7) not null (Year and month as 'YYYY-MM'.)
- `day_of_month` smallint not null (Day of the month, 1-31.)
- `day_of_year` smallint not null (Day of the year, 1-366.)
- `iso_year` integer not null (ISO 8601 week-numbering year (differs from year_num around New Year).)
- `iso_week` smallint not null (ISO 8601 week number, 1-53.)
- `iso_day_of_week` smallint not null (ISO day of the week: 1 = Monday ... 7 = Sunday.)
- `day_name` varchar(9) not null (English day name, e.g. 'Monday'.)
- `is_weekend` boolean not null (Saturday or Sunday.)
- `week_start` date not null (Monday of the ISO week.)
- `month_start` date not null (First day of the month.)
- `quarter_start` date not null (First day of the quarter.)

### locales
- `id` bigint not null
- `code` varchar(5) not null
- `name` varchar(255) not null
- `native_name` varchar(255)
- unique: (code)

### timezones
- `id` bigint not null
- `name` varchar(255) not null
- `utc_offset_minutes` integer not null
- unique: (name)

### addresses
Postal addresses owned by a tenant and optionally a user. At most one default address per user.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `label` varchar(255)
- `line1` varchar(255) not null
- `line2` varchar(255)
- `city` varchar(255)
- `region_id` bigint -> regions
- `country_id` bigint not null -> countries
- `postal_code` varchar(64)
- `latitude` decimal(9,6)
- `longitude` decimal(9,6)
- `is_default` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### address_validations
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `address_id` bigint not null -> addresses
- `provider` varchar(64) not null
- `is_valid` boolean not null
- `response` json
- `validated_at` timestamp not null

### postal_zones
- `id` bigint not null
- `country_id` bigint not null -> countries
- `code` varchar(64) not null
- `name` varchar(255) not null
- unique: (country_id, code)

## Area: identity

### tenants
Customer organizations on the platform; the root of multi-tenancy. Every tenant-scoped row hangs off a tenant_id.
- `id` bigint not null
- `public_id` uuid not null
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `status` varchar(32) not null one of: trial, active, past_due, suspended, churned
- `default_currency_code` varchar(3) not null -> currencies (ISO-4217 default currency.)
- `default_locale` varchar(5) not null -> locales
- `industry` varchar(255)
- `employee_count` integer
- `trial_ends_at` timestamp
- `signup_source` varchar(64)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (public_id); (slug)

### tenant_settings
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `setting_key` varchar(64) not null
- `setting_value` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, setting_key)

### users
End users and staff belonging to a tenant. Email is unique per tenant, not globally.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `email` varchar(254) not null
- `full_name` varchar(255)
- `status` varchar(32) not null one of: active, invited, suspended, deactivated
- `is_staff` boolean not null (True for internal/admin operators of the tenant, as opposed to customer end-users.)
- `locale` varchar(5) -> locales
- `timezone` varchar(64)
- `password_hash` varchar(255)
- `last_login_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (public_id); (tenant_id, email)

### user_profiles
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `avatar_url` varchar(2048)
- `bio` varchar(4000)
- `phone` varchar(32)
- `job_title` varchar(255)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (user_id)

### user_preferences
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `theme` varchar(64) not null
- `email_opt_in` boolean not null
- `marketing_opt_in` boolean not null
- `locale_override` varchar(5)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (user_id)

### roles
Per-tenant RBAC roles. is_system marks built-in roles that cannot be deleted.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `is_system` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### permissions
Global catalog of permission codes (e.g. orders.refund). Shared across all tenants.
- `id` bigint not null
- `code` varchar(64) not null
- `description` varchar(4000)
- `category` varchar(255)
- unique: (code)

### role_permissions
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `role_id` bigint not null -> roles
- `permission_id` bigint not null -> permissions
- `created_at` timestamp not null
- unique: (role_id, permission_id)

### user_roles
Assignment of roles to users. granted_by_user_id records who made the grant.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `role_id` bigint not null -> roles
- `granted_by_user_id` bigint -> users
- `created_at` timestamp not null
- unique: (user_id, role_id)

### teams
teams — hierarchical sub-org groupings.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `parent_team_id` bigint -> teams
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### team_members
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `team_id` bigint not null -> teams
- `user_id` bigint not null -> users
- `team_role` varchar(64) not null
- `created_at` timestamp not null
- unique: (team_id, user_id)

### api_keys
Programmatic API credentials for a tenant. Only the hash and a short prefix are stored.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `prefix` varchar(64) not null
- `hashed_key` varchar(64) not null
- `status` varchar(32) not null one of: active, revoked, expired
- `created_by_user_id` bigint -> users
- `last_used_at` timestamp
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id); (prefix)

### personal_access_tokens
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `name` varchar(255) not null
- `hashed_token` varchar(255) not null
- `scopes` varchar(64)[] not null
- `last_used_at` timestamp
- `expires_at` timestamp
- `created_at` timestamp not null
- unique: (hashed_token)

### sessions
Authenticated browser/app sessions. revoked_at non-null means force-logged-out.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `token_hash` varchar(255) not null
- `ip_address` varchar(45)
- `user_agent` varchar(2048)
- `started_at` timestamp not null
- `last_seen_at` timestamp not null
- `expires_at` timestamp not null
- `revoked_at` timestamp
- unique: (token_hash)

### oauth_accounts
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `provider` varchar(64) not null
- `provider_account_id` varchar(255) not null
- `access_token_hash` varchar(255)
- `created_at` timestamp not null
- unique: (provider, provider_account_id)

### mfa_devices
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `kind` varchar(64) not null
- `secret_hash` varchar(255) not null
- `confirmed_at` timestamp
- `created_at` timestamp not null

### invitations
Pending invitations to join a tenant; consumed when status becomes accepted.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `email` varchar(254) not null
- `role_id` bigint -> roles
- `invited_by_user_id` bigint -> users
- `status` varchar(32) not null one of: pending, accepted, expired, revoked
- `token_hash` varchar(255) not null
- `expires_at` timestamp not null
- `accepted_at` timestamp
- `created_at` timestamp not null
- unique: (token_hash)

### password_resets
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `token_hash` varchar(255) not null
- `expires_at` timestamp not null
- `used_at` timestamp
- `created_at` timestamp not null
- unique: (token_hash)

### service_accounts
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### login_attempts
Audit of authentication attempts, successful and failed, for security analytics.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `email` varchar(254)
- `succeeded` boolean not null
- `ip_address` varchar(45)
- `user_agent` varchar(2048)
- `occurred_at` timestamp not null

## Area: inventory

### warehouses
Physical fulfillment sites for a tenant, each anchored to a postal address; one is flagged as the default.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `address_id` bigint -> addresses
- `code` varchar(64) not null
- `name` varchar(255) not null
- `is_default` boolean not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, code)

### stock_locations
Bins, shelves and zones inside a warehouse. Stock is counted at this granularity, not at the warehouse level.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `warehouse_id` bigint not null -> warehouses
- `code` varchar(64) not null
- `name` varchar(255)
- `kind` varchar(64) not null
- `is_pickable` boolean not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (warehouse_id, code)

### suppliers
Vendors that goods are purchased from on purchase orders.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `code` varchar(64) not null
- `name` varchar(255) not null
- `contact_email` varchar(254)
- `contact_phone` varchar(32)
- `lead_time_days` integer
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, code)

### supplier_products
Per-supplier sourcing of a variant: their SKU and unit cost; is_preferred marks the default source.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `supplier_id` bigint not null -> suppliers
- `variant_id` bigint not null -> product_variants
- `supplier_sku` varchar(64)
- `unit_cost` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `min_order_qty` integer
- `lead_time_days` integer
- `is_preferred` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (supplier_id, variant_id)

### stock_items
Live on-hand quantity of a variant in a single location; one row per (variant, location). The movements ledger is its history.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `variant_id` bigint not null -> product_variants
- `stock_location_id` bigint not null -> stock_locations
- `quantity` integer not null
- `reserved_quantity` integer not null (Portion of quantity earmarked by open reservations and not available to sell.)
- `reorder_point` integer (Replenishment trigger: at or below this on-hand level the variant is flagged for reorder.)
- `reorder_quantity` integer
- `last_counted_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (variant_id, stock_location_id)

### stock_movements
Immutable append-only ledger of inventory quantity changes; quantity is signed (in positive, out negative) and the running sum reproduces stock_items.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `variant_id` bigint not null -> product_variants
- `stock_location_id` bigint not null -> stock_locations
- `lot_id` bigint -> lots
- `kind` varchar(32) not null one of: receipt, shipment, adjustment, transfer, return, cycle_count
- `quantity` integer not null
- `reference_type` varchar(64)
- `reference_id` bigint (Loose pointer to the source document (PO line, order, adjustment) identified by reference_type; not a foreign key.)
- `note` varchar(4000)
- `created_by_user_id` bigint -> users
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### stock_reservations
Soft holds that earmark stock for an order so it is not double-sold; order_id references orders by id but has no FK (forward module).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `variant_id` bigint not null -> product_variants
- `stock_location_id` bigint not null -> stock_locations
- `order_id` bigint -> orders
- `quantity` integer not null
- `expires_at` timestamp
- `released_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### purchase_orders
Replenishment orders sent to a supplier and delivered to a warehouse; public_id is the opaque external identifier.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `supplier_id` bigint not null -> suppliers
- `warehouse_id` bigint -> warehouses
- `po_number` varchar(64) not null
- `status` varchar(32) not null one of: draft, submitted, partially_received, received, cancelled
- `currency_code` varchar(3) not null -> currencies
- `expected_total` decimal(14,4) not null (Denormalized order total; the authoritative figure is the sum over purchase_order_lines.)
- `notes` varchar(4000)
- `ordered_at` timestamp
- `expected_at` timestamp
- `received_at` timestamp
- `created_by_user_id` bigint -> users
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id); (tenant_id, po_number)

### purchase_order_lines
Line items on a purchase order: variant, quantity and agreed unit cost, with received quantity accruing as goods arrive.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `purchase_order_id` bigint not null -> purchase_orders
- `variant_id` bigint not null -> product_variants
- `quantity_ordered` integer not null
- `quantity_received` integer not null
- `unit_cost` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (purchase_order_id, variant_id)

### lots
Batch/lot tracking for a variant, carrying manufacture and expiry dates for FEFO picking and recalls.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `variant_id` bigint not null -> product_variants
- `supplier_id` bigint -> suppliers
- `lot_code` varchar(64) not null
- `manufactured_at` date
- `expires_at` date
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, variant_id, lot_code)

### serial_numbers
serial_numbers — individually tracked units (one row per physical unit), optionally grouped under a lot and located in a stock location.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `variant_id` bigint not null -> product_variants
- `lot_id` bigint -> lots
- `stock_location_id` bigint -> stock_locations
- `serial` varchar(64) not null
- `status` varchar(64) not null
- `received_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, serial)

### inbound_receipts
Goods-receipt headers recording a physical delivery into a warehouse, usually against a purchase order.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `purchase_order_id` bigint -> purchase_orders
- `warehouse_id` bigint not null -> warehouses
- `receipt_number` varchar(64) not null
- `carrier` varchar(64)
- `tracking_number` varchar(64)
- `received_by_user_id` bigint -> users
- `received_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, receipt_number)

### receipt_lines
receipt_lines — the quantities of each variant actually received on a receipt, put away into a specific location (and optionally a lot).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `inbound_receipt_id` bigint not null -> inbound_receipts
- `purchase_order_line_id` bigint -> purchase_order_lines
- `variant_id` bigint not null -> product_variants
- `stock_location_id` bigint -> stock_locations
- `lot_id` bigint -> lots
- `quantity_received` integer not null
- `quantity_rejected` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### stock_adjustments
Manual corrections to on-hand stock (cycle counts, shrinkage, damage); quantity_delta is signed and a reason is required.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `variant_id` bigint not null -> product_variants
- `stock_location_id` bigint not null -> stock_locations
- `lot_id` bigint -> lots
- `quantity_delta` integer not null
- `reason` varchar(4000) not null
- `note` varchar(4000)
- `adjusted_by_user_id` bigint -> users
- `created_at` timestamp not null
- `updated_at` timestamp not null

## Area: marketing

### email_templates
Reusable email subject/body templates used by campaigns and per-recipient sends.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `subject` varchar(255) not null
- `body_html` varchar(65535)
- `body_text` varchar(65535)
- `from_name` varchar(255)
- `from_email` varchar(254)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### segments
Audience definitions; the selection logic lives in the rules jsonb and is materialized into segment_members.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `rules` json not null
- `is_dynamic` boolean not null (True if membership is recomputed from rules on a schedule; false for static lists.)
- `member_count` integer not null
- `last_refreshed_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### segment_members
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `segment_id` bigint not null -> segments
- `user_id` bigint not null -> users
- `added_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (segment_id, user_id)

### landing_pages
landing_pages — campaign destination pages. Referenced by campaigns and utm_links, so defined ahead of them.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `slug` varchar(255) not null
- `title` varchar(255) not null
- `url` varchar(2048)
- `content_html` varchar(65535)
- `is_published` boolean not null
- `published_at` timestamp
- `view_count` bigint not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, slug)

### campaigns
Marketing campaigns; may target a segment, render via a template, drive to a landing page, and link a pricing promotion.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `channel` varchar(64) not null
- `status` varchar(32) not null one of: draft, scheduled, sending, sent, paused, archived
- `segment_id` bigint -> segments
- `email_template_id` bigint -> email_templates
- `landing_page_id` bigint -> landing_pages
- `promotion_id` bigint -> promotions
- `budget` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `scheduled_at` timestamp
- `started_at` timestamp
- `ended_at` timestamp
- `created_by_user_id` bigint -> users
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### campaign_messages
One row per recipient send; send_status tracks the delivery and engagement lifecycle.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `campaign_id` bigint not null -> campaigns
- `user_id` bigint not null -> users
- `email_template_id` bigint -> email_templates
- `status` varchar(32) not null one of: queued, sent, delivered, opened, clicked, bounced, complained, failed, unsubscribed
- `recipient_email` varchar(254)
- `sent_at` timestamp
- `delivered_at` timestamp
- `opened_at` timestamp
- `clicked_at` timestamp
- `bounced_at` timestamp
- `unsubscribed_at` timestamp
- `error_message` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null

### ab_tests
A/B test definitions; winner_variant_id records the chosen variant once the experiment completes.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `campaign_id` bigint -> campaigns
- `name` varchar(255) not null
- `hypothesis` varchar(4000)
- `status` varchar(32) not null one of: draft, running, paused, completed, archived
- `goal_metric` varchar(255)
- `started_at` timestamp
- `ended_at` timestamp
- `winner_variant_id` bigint
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### ab_variants
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ab_test_id` bigint not null -> ab_tests
- `name` varchar(255) not null
- `is_control` boolean not null
- `traffic_weight` decimal(5,4) not null (Fraction of eligible traffic routed to this variant; weights across a test should sum to ~1.)
- `config` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (ab_test_id, name)

### ab_assignments
Sticky assignment of a user to one variant within an A/B test; converted_at records goal completion.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ab_test_id` bigint not null -> ab_tests
- `ab_variant_id` bigint not null -> ab_variants
- `user_id` bigint not null -> users
- `assigned_at` timestamp not null
- `converted_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (ab_test_id, user_id)

### attributions
Touch attributions crediting a campaign with a conversion; order_id is set for purchase conversions.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `campaign_id` bigint -> campaigns
- `order_id` bigint -> orders
- `model` varchar(64) not null
- `touch_point` varchar(64)
- `weight` decimal(5,4) not null (Fractional credit for this touch under the attribution model (e.g. linear, last_touch).)
- `attributed_revenue` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `occurred_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### referrals
Referral codes owned by a referring user, with optional reward and redemption caps.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `referrer_user_id` bigint not null -> users
- `code` varchar(64) not null
- `reward_amount` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `max_redemptions` integer
- `redemption_count` integer not null
- `is_active` boolean not null
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, code)

### referral_redemptions
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `referral_id` bigint not null -> referrals
- `referred_user_id` bigint not null -> users
- `order_id` bigint -> orders
- `reward_amount` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `redeemed_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (referral_id, referred_user_id)

### loyalty_programs
Loyalty: program defs -> per-user accounts (balance) -> immutable earn/burn transaction ledger.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `points_per_currency` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `is_active` boolean not null
- `starts_at` timestamp
- `ends_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, name)

### loyalty_accounts
Per-user point balance within a loyalty program; points_balance is the current redeemable total.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `loyalty_program_id` bigint not null -> loyalty_programs
- `user_id` bigint not null -> users
- `points_balance` bigint not null
- `lifetime_points` bigint not null
- `tier` varchar(64)
- `enrolled_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (loyalty_program_id, user_id)

### loyalty_transactions
Immutable earn/burn ledger of loyalty point movements; append-only, never updated.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `loyalty_account_id` bigint not null -> loyalty_accounts
- `user_id` bigint not null -> users
- `direction` varchar(64) not null
- `points` bigint not null
- `reason` varchar(4000)
- `order_id` bigint -> orders
- `balance_after` bigint (Account points_balance immediately after this entry was applied.)
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### utm_links
Tracked links carrying UTM parameters, optionally tied to a campaign and landing page.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `campaign_id` bigint -> campaigns
- `landing_page_id` bigint -> landing_pages
- `slug` varchar(255) not null
- `target_url` varchar(2048) not null
- `utm_source` varchar(255)
- `utm_medium` varchar(255)
- `utm_campaign` varchar(255)
- `utm_term` varchar(255)
- `utm_content` varchar(255)
- `click_count` bigint not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, slug)

## Area: ops

### feature_flags
Feature toggle and rollout definitions, addressed by a tenant-unique key. kind distinguishes boolean toggles from multivariate and percentage rollouts.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `key` varchar(64) not null
- `name` varchar(255) not null
- `description` varchar(4000)
- `kind` varchar(32) not null one of: boolean, multivariate, percentage_rollout
- `is_enabled` boolean not null
- `default_value` json not null (Fallback value served when no rule or override matches; jsonb to support multivariate payloads, not just booleans.)
- `rollout_percentage` integer
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, key)

### feature_flag_rules
Ordered targeting rules for a flag; the first rule whose jsonb conditions match decides the served value.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `feature_flag_id` bigint not null -> feature_flags
- `sort_position` integer not null
- `conditions` json not null
- `serve_value` json not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (feature_flag_id, sort_position)

### feature_flag_overrides
Explicit per-user or tenant-wide overrides that bypass rule evaluation for a flag.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `feature_flag_id` bigint not null -> feature_flags
- `user_id` bigint -> users
- `override_value` json not null
- `reason` varchar(4000)
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### settings
Typed key/value configuration per tenant. value_type tags how the jsonb value should be interpreted by the application.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `setting_key` varchar(64) not null
- `setting_value` json not null
- `value_type` varchar(64) not null (Logical type hint for value (string/number/boolean/json/date); the column itself is always jsonb.)
- `is_secret` boolean not null
- `description` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, setting_key)

### jobs
Background job definitions registered per tenant; job_runs records each execution.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `queue` varchar(64) not null
- `handler` varchar(64) not null
- `payload` json not null
- `max_attempts` integer not null
- `is_enabled` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### job_runs
Immutable record of a single background job execution, including status, attempt number and timing.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `job_id` bigint not null -> jobs
- `status` varchar(32) not null one of: queued, running, succeeded, failed, cancelled, retrying
- `attempt` integer not null
- `queued_at` timestamp not null
- `started_at` timestamp
- `finished_at` timestamp
- `duration_ms` integer
- `error_message` varchar(4000)
- `result_payload` json
- `created_at` timestamp not null

### integrations
Third-party service connections for a tenant (e.g. ERP, shipping, accounting), with current status and config.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `provider` varchar(64) not null
- `name` varchar(255) not null
- `status` varchar(32) not null one of: connected, disconnected, error, syncing
- `config` json not null
- `connected_by_user_id` bigint -> users
- `connected_at` timestamp
- `last_synced_at` timestamp
- `last_error` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, provider, name)

### integration_syncs
Immutable record of a single data-sync run against an integration.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `integration_id` bigint not null -> integrations
- `status` varchar(32) not null one of: queued, running, succeeded, failed, cancelled, retrying
- `direction` varchar(64) not null
- `records_processed` integer not null
- `records_failed` integer not null
- `started_at` timestamp
- `finished_at` timestamp
- `error_message` varchar(4000)
- `created_at` timestamp not null

### files
Metadata for files uploaded to object storage; the actual blob lives in storage_bucket/storage_key, not in the database.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `filename` varchar(255) not null
- `content_type` varchar(64) not null
- `byte_size` bigint not null
- `storage_bucket` varchar(255) not null
- `storage_key` varchar(2048) not null
- `checksum` varchar(255)
- `uploaded_by_user_id` bigint -> users
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (public_id); (storage_bucket, storage_key)

### file_versions
Version chain for a file; is_current flags the active version (one per file, enforced by partial unique index).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `file_id` bigint not null -> files
- `version` integer not null
- `byte_size` bigint not null
- `storage_key` varchar(2048) not null
- `checksum` varchar(255)
- `is_current` boolean not null
- `created_by_user_id` bigint -> users
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (file_id, version)

### import_batches
Bulk data-import runs with row-level progress counters and a terminal status.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `entity_type` varchar(64) not null
- `source_filename` varchar(255)
- `file_id` bigint -> files
- `status` varchar(32) not null one of: pending, processing, completed, failed, partially_failed
- `total_rows` integer not null
- `processed_rows` integer not null
- `succeeded_rows` integer not null
- `failed_rows` integer not null
- `started_by_user_id` bigint -> users
- `started_at` timestamp
- `finished_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### import_errors
Immutable per-row failures captured during an import batch, with the offending raw row preserved as jsonb.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `import_batch_id` bigint not null -> import_batches
- `source_row_number` integer not null
- `column_name` varchar(64)
- `error_code` varchar(64) not null
- `error_message` varchar(4000) not null
- `raw_row` json
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### scheduled_tasks
Cron-expression driven recurring tasks; next_run_at is the precomputed next fire time used by the scheduler.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `cron_expression` varchar(2048) not null
- `timezone` varchar(64) not null
- `job_id` bigint -> jobs
- `is_enabled` boolean not null
- `last_run_at` timestamp
- `next_run_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### system_health_checks
system_health_checks — immutable periodic health probe results.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `component` varchar(255) not null
- `is_healthy` boolean not null
- `latency_ms` integer
- `details` json
- `checked_at` timestamp not null
- `created_at` timestamp not null

### rate_limits
Request quota windows scoped per tenant or per API key; an api_key-null row is the tenant-wide default for a scope.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `api_key_id` bigint -> api_keys
- `limit_scope` varchar(64) not null
- `max_requests` integer not null
- `window_seconds` integer not null
- `is_enabled` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### secrets_vault
References and metadata for secrets held in an external vault; never stores plaintext, only a provider pointer (external_ref).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `key` varchar(64) not null
- `provider` varchar(64) not null
- `external_ref` varchar(255) not null (Opaque pointer into the external secret store (e.g. a Vault path or KMS key id); resolved at runtime, never the secret value itself.)
- `version` integer not null
- `last_rotated_at` timestamp
- `expires_at` timestamp
- `created_by_user_id` bigint -> users
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, key)

## Area: pricing

### price_lists
Named price books, each scoped to one currency; a tenant keeps a default list per currency plus contract/regional books.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `code` varchar(64) not null
- `currency_code` varchar(3) not null -> currencies
- `is_default` boolean not null
- `is_active` boolean not null
- `starts_at` timestamp
- `ends_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, code)

### prices
A variant's price within a price list over a validity window; an exclusion constraint forbids overlapping windows for the same list/variant/min_quantity.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `price_list_id` bigint not null -> price_lists
- `variant_id` bigint not null -> product_variants
- `currency_code` varchar(3) not null -> currencies
- `amount` decimal(14,4) not null
- `compare_at_amount` decimal(14,4) (Optional strike-through / "was" price shown next to the effective amount.)
- `min_quantity` integer not null
- `valid_from` timestamp not null
- `valid_to` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### discounts
Reusable discount definitions; coupons, price_rules and promotions point at these for the actual reduction.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `kind` varchar(32) not null one of: percentage, fixed_amount, free_shipping, bogo
- `discount_value` decimal(14,4) not null (Percentage (0-100) when kind=percentage, else a fixed monetary amount in currency_code.)
- `currency_code` varchar(3) -> currencies
- `max_amount` decimal(14,4)
- `min_subtotal` decimal(14,4)
- `applies_to_shipping` boolean not null
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp

### coupons
Redeemable codes granting a discount; code is unique per tenant and usage is capped by usage_limit / per_customer_limit.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `discount_id` bigint not null -> discounts
- `code` varchar(64) not null
- `status` varchar(32) not null one of: active, scheduled, expired, disabled
- `usage_limit` integer
- `per_customer_limit` integer
- `times_redeemed` integer not null (Denormalized running count of redemptions, capped by usage_limit.)
- `starts_at` timestamp
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, code)

### coupon_redemptions
Immutable record of each coupon use; order_id references orders.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `coupon_id` bigint not null -> coupons
- `user_id` bigint -> users
- `order_id` bigint -> orders
- `discount_amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `redeemed_at` timestamp not null
- `created_at` timestamp not null

### tax_categories
tax_categories — product tax classes (e.g. standard goods, digital, food, exempt). Variants/products are mapped to one of these to find their rate.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `code` varchar(64) not null
- `description` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, code)

### tax_zones
Geographic tax regions defined at country, region or postal-zone granularity.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `country_id` bigint -> countries
- `region_id` bigint -> regions
- `postal_zone_id` bigint -> postal_zones
- `created_at` timestamp not null
- `updated_at` timestamp not null

### tax_rates
Effective tax rate for a (tax_category, tax_zone) pair, with an optional validity window so rate changes stay auditable.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `tax_category_id` bigint not null -> tax_categories
- `tax_zone_id` bigint not null -> tax_zones
- `name` varchar(255) not null
- `rate` decimal(8,5) not null (Fractional rate in [0,1]; e.g. 0.20000 for 20% VAT.)
- `is_compound` boolean not null
- `is_inclusive` boolean not null (True when displayed prices already include this tax (tax-inclusive pricing).)
- `valid_from` timestamp not null
- `valid_to` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### price_rules
price_rules — conditional pricing rule headers (e.g. "10% off when cart has 3+ items from brand X"). Conditions live in price_rule_conditions. The reward references a discount definition. priority orders evaluation; lower = first.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `discount_id` bigint -> discounts
- `name` varchar(255) not null
- `priority` integer not null
- `is_active` boolean not null
- `stackable` boolean not null
- `starts_at` timestamp
- `ends_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### price_rule_conditions
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `price_rule_id` bigint not null -> price_rules
- `attribute` varchar(255) not null
- `operator` varchar(64) not null
- `condition_value` json not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### promotions
Campaign-linked promotions; standalone by design so campaigns can reference promotions, never the reverse.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `discount_id` bigint -> discounts
- `coupon_id` bigint -> coupons
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `campaign_code` varchar(64) (Free-text correlation handle to a marketing campaign; not an FK since marketing is a higher-numbered module.)
- `is_active` boolean not null
- `starts_at` timestamp
- `ends_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, slug)

### promotion_products
Junction restricting a promotion to specific products.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `promotion_id` bigint not null -> promotions
- `product_id` bigint not null -> products
- `created_at` timestamp not null
- unique: (promotion_id, product_id)

## Area: sales

### carts
Shopping carts, anonymous (user_id null) or owned. Converts to an order; abandoned carts feed recovery campaigns.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `session_token` varchar(255)
- `status` varchar(32) not null one of: active, converted, abandoned, merged
- `currency_code` varchar(3) not null -> currencies
- `item_count` integer not null
- `subtotal` decimal(14,4) not null
- `abandoned_at` timestamp
- `converted_at` timestamp
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### cart_items
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `cart_id` bigint not null -> carts
- `product_variant_id` bigint not null -> product_variants
- `quantity` integer not null
- `unit_price` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (cart_id, product_variant_id)

### orders
Central order fact table. public_id is the opaque id shown to customers; totals are denormalized money on the row.
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `cart_id` bigint -> carts
- `order_number` varchar(64) not null (Human-facing sequential order number, unique per tenant (distinct from the opaque public_id).)
- `status` varchar(32) not null one of: pending, confirmed, paid, partially_fulfilled, fulfilled, cancelled, refunded
- `billing_address_id` bigint -> addresses
- `shipping_address_id` bigint -> addresses
- `currency_code` varchar(3) not null -> currencies
- `subtotal` decimal(14,4) not null
- `discount_total` decimal(14,4) not null
- `tax_total` decimal(14,4) not null
- `shipping_total` decimal(14,4) not null
- `grand_total` decimal(14,4) not null (Final amount charged: subtotal - discount_total + tax_total + shipping_total.)
- `customer_email` varchar(254)
- `customer_note` varchar(4000)
- `placed_at` timestamp
- `cancelled_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id); (tenant_id, order_number)

### order_items
Line items of an order; sku and name are snapshotted at purchase time so historical orders survive catalog changes.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `order_id` bigint not null -> orders
- `product_variant_id` bigint not null -> product_variants
- `sku` varchar(64) not null
- `name` varchar(255) not null
- `quantity` integer not null
- `quantity_fulfilled` integer not null (Running count of units already dispatched across fulfillments; never exceeds quantity.)
- `unit_price` decimal(14,4) not null
- `discount_total` decimal(14,4) not null
- `tax_total` decimal(14,4) not null
- `line_total` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null

### order_status_history
Append-only audit of order status transitions; immutable (no updated_at).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `order_id` bigint not null -> orders
- `from_status` varchar(32) one of: pending, confirmed, paid, partially_fulfilled, fulfilled, cancelled, refunded
- `to_status` varchar(32) not null one of: pending, confirmed, paid, partially_fulfilled, fulfilled, cancelled, refunded
- `changed_by_user_id` bigint -> users
- `note` varchar(4000)
- `created_at` timestamp not null

### order_discounts
Discounts and coupon redemptions applied to an order; discount_id/coupon_id reference the pricing module.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `order_id` bigint not null -> orders
- `discount_id` bigint -> discounts
- `coupon_id` bigint -> coupons
- `code` varchar(64)
- `description` varchar(4000)
- `amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null

### fulfillments
A batch of order items dispatched together from one warehouse; tracks pick/pack/ship lifecycle.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `order_id` bigint not null -> orders
- `warehouse_id` bigint -> warehouses
- `status` varchar(32) not null one of: pending, picked, packed, shipped, delivered, returned
- `reference` varchar(255)
- `packed_at` timestamp
- `shipped_at` timestamp
- `delivered_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### fulfillment_items
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `fulfillment_id` bigint not null -> fulfillments
- `order_item_id` bigint not null -> order_items
- `quantity` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (fulfillment_id, order_item_id)

### shipments
Carrier shipment for a fulfillment, with tracking number and delivery timestamps.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `fulfillment_id` bigint not null -> fulfillments
- `carrier` varchar(64) not null
- `service_level` varchar(64)
- `tracking_number` varchar(64)
- `tracking_url` varchar(2048)
- `shipping_cost` decimal(14,4)
- `currency_code` varchar(3) -> currencies
- `weight_grams` integer
- `shipped_at` timestamp
- `delivered_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

### shipment_tracking
shipment_tracking — append-only stream of carrier tracking events. IMMUTABLE: occurred_at only, no updated_at.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `shipment_id` bigint not null -> shipments
- `status` varchar(64) not null
- `description` varchar(4000)
- `location` varchar(255)
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### order_returns
Return merchandise authorization (RMA) header; status walks requested -> approved -> received -> refunded.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `order_id` bigint not null -> orders
- `rma_number` varchar(64) not null
- `status` varchar(32) not null one of: requested, approved, received, refunded, rejected
- `reason` varchar(4000)
- `refund_total` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `requested_at` timestamp not null
- `approved_at` timestamp
- `received_at` timestamp
- `refunded_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, rma_number)

### return_items
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `return_id` bigint not null -> order_returns
- `order_item_id` bigint not null -> order_items
- `quantity` integer not null
- `reason` varchar(4000)
- `restock` boolean not null (Whether the returned units should be added back to sellable inventory.)
- `refund_amount` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (return_id, order_item_id)

### gift_cards
Stored-value gift cards; code is unique per tenant and balance decreases as it is redeemed against orders.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `code` varchar(64) not null
- `status` varchar(64) not null
- `initial_balance` decimal(14,4) not null
- `balance` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `issued_to_user_id` bigint -> users
- `issued_order_id` bigint -> orders
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, code)

### gift_card_transactions
Immutable ledger of gift-card balance changes; positive amount credits, negative debits.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `gift_card_id` bigint not null -> gift_cards
- `order_id` bigint -> orders
- `amount` decimal(14,4) not null
- `balance_after` decimal(14,4) not null
- `currency_code` varchar(3) not null -> currencies
- `note` varchar(4000)
- `created_at` timestamp not null

### order_notes
order_notes — internal staff notes attached to an order (not the customer note that lives on the order row itself).
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `order_id` bigint not null -> orders
- `author_user_id` bigint -> users
- `body` varchar(65535) not null
- `is_pinned` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null

### draft_orders
Merchant-created quotes/draft orders, optionally converted into a real order.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint -> users
- `created_by_user_id` bigint -> users
- `converted_order_id` bigint -> orders
- `name` varchar(255)
- `status` varchar(64) not null
- `currency_code` varchar(3) not null -> currencies
- `subtotal` decimal(14,4) not null
- `grand_total` decimal(14,4) not null
- `payload` json not null
- `expires_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null

## Area: support

### support_agents
Support staff who handle tickets; a support-specific extension of an users record carrying availability and load limits.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `user_id` bigint not null -> users
- `display_name` varchar(255)
- `signature` varchar(255)
- `is_available` boolean not null
- `max_open_tickets` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, user_id)

### agent_groups
Named groups of agents used for ticket routing and assignment.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `description` varchar(4000)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### agent_group_members
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `agent_group_id` bigint not null -> agent_groups
- `support_agent_id` bigint not null -> support_agents
- `is_lead` boolean not null
- `created_at` timestamp not null
- unique: (agent_group_id, support_agent_id)

### sla_policies
Lookup of service-level targets (first-response and resolution minutes), usually keyed by ticket priority.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `priority` varchar(32) one of: low, normal, high, urgent
- `first_response_target_minutes` integer not null
- `resolution_target_minutes` integer not null
- `business_hours_only` boolean not null (When true, SLA timers pause outside the tenant business calendar rather than running 24/7.)
- `is_active` boolean not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### support_tags
support_tags — tag catalog for categorizing tickets.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `color` varchar(64)
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, name)

### knowledge_categories
Knowledge base: hierarchical categories + articles + per-article feedback.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `name` varchar(255) not null
- `slug` varchar(255) not null
- `parent_category_id` bigint -> knowledge_categories
- `sort_position` integer not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (tenant_id, slug)

### knowledge_articles
Help-center articles published to customers; soft-deletable via deleted_at.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `knowledge_category_id` bigint -> knowledge_categories
- `author_user_id` bigint -> users
- `title` varchar(255) not null
- `slug` varchar(255) not null
- `body` varchar(65535)
- `is_published` boolean not null
- `view_count` bigint not null
- `helpful_count` integer not null (Denormalized count of positive article_feedback rows, kept for fast sorting.)
- `not_helpful_count` integer not null
- `published_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, slug)

### article_feedback
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `knowledge_article_id` bigint not null -> knowledge_articles
- `user_id` bigint -> users
- `was_helpful` boolean not null
- `comment` varchar(4000)
- `created_at` timestamp not null
- `updated_at` timestamp not null

### macros
Canned reply templates agents insert into ticket messages. Soft-deletable.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `created_by_agent_id` bigint -> support_agents
- `title` varchar(255) not null
- `body` varchar(65535) not null
- `is_shared` boolean not null
- `usage_count` bigint not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- `deleted_at` timestamp
- unique: (tenant_id, title)

### tickets
Core customer support request. requester_user_id is the customer; assignee_agent_id is the owning agent (null = unassigned queue).
- `id` bigint not null
- `public_id` uuid not null
- `tenant_id` bigint not null -> tenants
- `requester_user_id` bigint not null -> users
- `assignee_agent_id` bigint -> support_agents
- `agent_group_id` bigint -> agent_groups
- `sla_policy_id` bigint -> sla_policies
- `subject` varchar(255) not null
- `status` varchar(32) not null one of: new, open, pending, on_hold, solved, closed
- `priority` varchar(32) not null one of: low, normal, high, urgent
- `channel` varchar(32) not null one of: email, chat, phone, web, social, api
- `first_response_at` timestamp (Timestamp of the first public agent reply, used to measure SLA first-response compliance.)
- `resolved_at` timestamp
- `closed_at` timestamp
- `last_message_at` timestamp
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (public_id)

### ticket_messages
Replies on a ticket: is_public=true are customer-visible, is_public=false are internal agent notes.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ticket_id` bigint not null -> tickets
- `author_user_id` bigint -> users
- `author_agent_id` bigint -> support_agents
- `body` varchar(65535) not null
- `is_public` boolean not null
- `channel` varchar(32) one of: email, chat, phone, web, social, api
- `created_at` timestamp not null
- `updated_at` timestamp not null

### ticket_events
Immutable append-only audit of ticket changes such as status transitions, reassignment, and priority changes.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ticket_id` bigint not null -> tickets
- `actor_user_id` bigint -> users
- `actor_agent_id` bigint -> support_agents
- `event_type` varchar(64) not null
- `old_value` varchar(4000)
- `new_value` varchar(4000)
- `metadata` json not null
- `occurred_at` timestamp not null
- `created_at` timestamp not null

### ticket_tags_map
ticket_tags_map — junction tickets × support_tags.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ticket_id` bigint not null -> tickets
- `support_tag_id` bigint not null -> support_tags
- `created_at` timestamp not null
- unique: (ticket_id, support_tag_id)

### sla_breaches
Recorded SLA-target breaches on a ticket, with how many minutes the target was missed by.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ticket_id` bigint not null -> tickets
- `sla_policy_id` bigint -> sla_policies
- `breach_kind` varchar(64) not null
- `target_at` timestamp not null
- `breached_at` timestamp not null
- `minutes_over` integer
- `created_at` timestamp not null
- `updated_at` timestamp not null

### csat_responses
Customer satisfaction rating (1-5) collected after a ticket is resolved; one response per ticket.
- `id` bigint not null
- `tenant_id` bigint not null -> tenants
- `ticket_id` bigint not null -> tickets
- `respondent_user_id` bigint -> users
- `rated_agent_id` bigint -> support_agents (The agent the customer was rating, captured at submission in case the ticket is later reassigned.)
- `score` integer not null
- `comment` varchar(4000)
- `submitted_at` timestamp not null
- `created_at` timestamp not null
- `updated_at` timestamp not null
- unique: (ticket_id)


# Your role

You are Ines Laurent, Data Protection Officer at Tidewick Commerce.