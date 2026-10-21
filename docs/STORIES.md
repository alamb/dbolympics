# Planted stories (never shown to the query-generation agents)

Things that "happened" at Tidewick, planted by the
simulator. Each must hold at every scale factor (same merchants, dates, names, effect sizes).
Merchants are in `world/merchants.json` (each lists its `stories`); carriers and payment
gateways named here are the ones assigned there.

Per story: **what**, **mechanism** (rows the simulator writes), **overview signal** (how a
broad report reveals that something is off), **drill-down** (the path an analyst would follow).

## Operations

### S1. UK carrier disruption (Nov-Dec 2025)
- **What**: carrier "Albion Parcel" (most UK shipments) has a strike / capacity crisis from
  2025-11-10 to 2025-12-08, peaking in Black Friday week.
- **Mechanism**: GB-bound shipments with that carrier deliver 5-12 days late; tracking shows
  `exception` / `delayed` events; "where is my order" tickets spike; cancellations and
  refunds rise; CSAT drops; affected shoppers buy less in Dec-Feb. One large apparel merchant
  switches to "Kennet Freight" on 2025-11-24, and its delays recover.
- **Overview signal**: Q4 revenue growth is weaker in the UK than elsewhere; UK CSAT drops in
  November.
- **Drill-down**: UK by week -> delivery times by carrier -> tickets and refunds per carrier
  -> merchants that switched vs didn't -> repeat purchase of affected shoppers.
- **Tables**: `shipments`, `shipment_tracking`, `fulfillments`, `fulfillment_items`, `orders`, `order_items`, `order_status_history`, `addresses`, `countries`, `regions`, `warehouses`, `tickets`, `ticket_messages`, `ticket_events`, `ticket_tags_map`, `support_tags`, `sla_breaches`, `csat_responses`, `payment_refunds`, `payments`, `notifications`, `sms_log`, `email_log`, `message_threads`, `thread_messages`, `tenants`

### S2. Payment gateway outage (2025-03-14)
- **What**: gateway "Halyard Pay" degrades from 06:10 to 14:40 UTC.
- **Mechanism**: `payment_attempts` fail with `gateway_timeout`; shoppers retry; some
  attempts are captured twice and later refunded with reason `duplicate`; abandoned carts
  spike for merchants using Halyard Pay only; merchants with a fallback gateway are unaffected.
- **Overview signal**: a March dip in payment success rate; one day with an odd revenue drop.
- **Drill-down**: success rate by day -> by gateway and hour -> duplicate refunds -> lost
  revenue per merchant -> merchants without a fallback gateway.
- **Tables**: `payments`, `payment_attempts`, `payment_refunds`, `payment_methods`, `carts`, `cart_items`, `orders`, `ledger_entries`, `ledger_accounts`, `payouts`, `payout_items`, `webhooks`, `webhook_deliveries`, `integrations`, `tenants`

### S3. Black Friday site outage (2025-11-28)
- **What**: platform-wide degradation 14:00-15:50 UTC.
- **Mechanism**: `system_health_checks` unhealthy, sessions bounce, carts abandoned,
  `job_runs` fail and retry, webhook deliveries fail and retry later.
- **Overview signal**: Black Friday 2025 underperforms Black Friday 2024 relative to the
  trend.
- **Drill-down**: Black Friday by hour vs last year -> bounce rate and cart abandonment ->
  health checks and failed jobs in the window.
- **Tables**: `system_health_checks`, `web_sessions`, `page_views`, `events`, `carts`, `cart_items`, `jobs`, `job_runs`, `scheduled_tasks`, `webhooks`, `webhook_deliveries`, `sessions`

### S4. Supplier lead-time blow-up (from July 2025)
- **What**: the preferred supplier of a large electronics merchant slips from ~14 to ~45
  days lead time.
- **Mechanism**: purchase orders arrive late; stock of top SKUs hits zero; cancellations and
  "out of stock" tickets rise; reviews mention delays; the merchant moves some SKUs to a
  second supplier in October.
- **Overview signal**: electronics revenue flattens in Q3 while traffic keeps growing (lower
  conversion).
- **Drill-down**: conversion by vertical -> stockouts by SKU -> PO lead times by supplier.
- **Tables**: `suppliers`, `supplier_products`, `purchase_orders`, `purchase_order_lines`, `inbound_receipts`, `receipt_lines`, `stock_items`, `stock_locations`, `stock_movements`, `stock_reservations`, `warehouses`, `product_variants`, `products`, `categories`, `orders`, `order_items`, `product_reviews`, `tickets`, `events`

### S5. Defective product lot (spring 2025)
- **What**: one lot of a popular kitchen appliance at a home-and-living merchant is defective.
- **Mechanism**: units from that lot get returned at ~35% vs ~5% normally. The lot is only
  traceable the messy way: shipment `stock_movements` carry `lot_id` and a
  `reference_type`/`reference_id` pointing at the order (there is no direct
  order_item -> lot link); review ratings for the product drop after April; recall stock
  adjustments in May.
- **Overview signal**: the home vertical's return rate jumps in April-May.
- **Drill-down**: returns by product -> by lot (via stock movements) -> reviews over time ->
  recall adjustments.
- **Tables**: `lots`, `serial_numbers`, `stock_movements`, `stock_adjustments`, `inbound_receipts`, `receipt_lines`, `order_returns`, `return_items`, `order_items`, `products`, `product_variants`, `product_reviews`, `review_votes`, `tickets`

## Business dynamics

### S6. Subscription price increase (2025-04-01)
- **What**: the largest coffee-subscription merchant raises prices ~18%.
- **Mechanism**: new `plan_prices`; monthly subscribers churn about 2.5x more for three
  months (more in Germany); annual subscribers only churn at renewal; win-back campaign in
  June partly works.
- **Overview signal**: subscription revenue holds up short-term, but active subscriber counts
  fall for one merchant.
- **Drill-down**: churn by merchant and month -> by plan interval and country -> before vs
  after the price change -> win-back campaign effect.
- **Tables**: `plans`, `plan_prices`, `plan_features`, `subscriptions`, `subscription_items`, `usage_records`, `invoices`, `invoice_lines`, `dunning_attempts`, `payments`, `campaigns`, `campaign_messages`, `segments`, `segment_members`, `email_templates`, `attributions`

### S7. Influencer campaign that buys bad customers (May 2025)
- **What**: an apparel merchant runs a "Spring Mega Sale" with an influencer: 40% coupon.
- **Mechanism**: a big spike in signups and first orders; those cohorts have high return
  rates (bracketing: buy 3 sizes, return 2), low repeat rates, and many unsubscribes.
- **Overview signal**: May is a record month for that merchant; net revenue after returns is
  disappointing.
- **Drill-down**: cohorts by acquisition campaign -> return rate and repeat rate by cohort ->
  coupon redemptions -> contribution after refunds.
- **Tables**: `campaigns`, `promotions`, `promotion_products`, `discounts`, `coupons`, `coupon_redemptions`, `order_discounts`, `utm_links`, `landing_pages`, `web_sessions`, `attributions`, `users`, `orders`, `order_returns`, `return_items`, `unsubscribes`, `consent_records`, `referrals`, `referral_redemptions`, `cohorts`, `cohort_members`

### S8. Coupon fraud ring (Aug-Oct 2025)
- **What**: ~300 accounts at a beauty merchant abuse new-customer coupons.
- **Mechanism**: accounts share ~20 shipping addresses and a few device fingerprints, use
  the welcome coupon once each, often pay with cards that share brand and last4, and have a
  high refund / chargeback share.
- **Overview signal**: new-customer share and coupon cost rise for one merchant.
- **Drill-down**: coupon redemptions per address / device -> clusters of accounts -> their
  refunds and payments.
- **Tables**: `coupons`, `coupon_redemptions`, `addresses`, `address_validations`, `device_profiles`, `payment_methods`, `payments`, `payment_refunds`, `users`, `login_attempts`, `sessions`, `compliance_events`, `audit_log`

### S9. Support understaffing (Aug-Sep 2025)
- **What**: a B2B wholesaler loses two of its five support agents.
- **Mechanism**: first response and resolution times double, SLA breaches rise, CSAT falls;
  two large B2B accounts with breached tickets reduce orders in Q4 (deals not renewed).
- **Overview signal**: the wholesaler's revenue drops in Q4 while other B2B merchants grow.
- **Drill-down**: revenue by account -> tickets and SLA breaches per account -> agent load
  over time.
- **Tables**: `support_agents`, `agent_groups`, `agent_group_members`, `teams`, `team_members`, `tickets`, `ticket_messages`, `ticket_events`, `sla_policies`, `sla_breaches`, `csat_responses`, `macros`, `knowledge_articles`, `article_feedback`, `accounts`, `contacts`, `opportunities`, `deals`, `deal_line_items`, `activities`, `crm_notes`, `tasks`, `invoices`, `draft_orders`, `billing_profiles`

### S10. Mobile app launch (2025-05-15, positive)
- **What**: a sports merchant launches a mobile app.
- **Mechanism**: push tokens, app sessions (device type, `properties.platform = 'ios' /
  'android'` in events), push notifications; app users repeat ~1.6x more often.
- **Overview signal**: that merchant's repeat rate rises after May.
- **Drill-down**: repeat rate by channel / device -> app vs web cohorts.
- **Tables**: `push_tokens`, `notifications`, `notification_templates`, `notification_preferences`, `device_profiles`, `web_sessions`, `events`, `feature_usage`, `loyalty_programs`, `loyalty_accounts`, `loyalty_transactions`, `users`, `user_preferences`

### S11. A/B test with a sample ratio mismatch (Sep 2025)
- **What**: a checkout experiment at a large merchant; an assignment bug sends returning
  users disproportionately to variant B.
- **Mechanism**: B gets ~58% of traffic instead of 50%, and more returning users; B looks
  like +9% conversion, but within new and within returning users it is flat.
- **Overview signal**: the experiment report says B wins.
- **Drill-down**: assignment counts by variant -> share of returning users per variant ->
  conversion within segments.
- **Tables**: `experiments`, `experiment_variants`, `experiment_assignments`, `funnels`, `funnel_steps`, `events`, `web_sessions`, `orders`, `feature_flags`, `feature_flag_rules`, `feature_flag_overrides`

## Data quality

### S12. Duplicated analytics events (2025-06-02 to 2025-06-10)
- **What**: an SDK release (`properties.sdk_version = '4.2.0'`) double-sends events from
  mobile web.
- **Mechanism**: duplicate events a few milliseconds apart with identical properties.
- **Overview signal**: events jump in June while orders do not, so "conversion" drops.
- **Drill-down**: events per session by day -> by SDK version -> near-duplicate pairs.
- **Tables**: `events`, `web_sessions`, `page_views`, `device_profiles`, `metrics_daily`, `kpi_snapshots`

### S13. Time-zone bug in an ERP integration (from 2024-10-27)
- **What**: one merchant's ERP sync writes local Berlin time into UTC timestamps for
  orders imported through the integration, starting at the DST change.
- **Mechanism**: those orders' `placed_at` is shifted 1-2 hours; that merchant's hourly
  pattern looks odd, and orders appear to be placed before their payments.
- **Overview signal**: hard to see in daily reports; visible in hour-of-day analyses and in
  "payment before order" checks.
- **Drill-down**: orders by hour per merchant -> payment-to-order time gaps -> integration
  source.
- **Tables**: `integrations`, `integration_syncs`, `orders`, `payments`, `order_status_history`, `audit_log`, `change_history`, `tenant_settings`, `settings`, `timezones`

### S14. Duplicate customers from a migration import (Feb 2024)
- **What**: a merchant re-imports its customer list (`import_batches`, `import_errors`).
- **Mechanism**: ~8% of its shoppers exist twice with email variants (plus-addressing, dots
  in gmail-style addresses); orders are split across the duplicates.
- **Overview signal**: that merchant's customer count and new-customer rate jump in Feb 2024;
  orders per customer drop.
- **Drill-down**: signups by day -> import batches -> customers sharing names / addresses
  with normalized email.
- **Tables**: `import_batches`, `import_errors`, `files`, `file_versions`, `users`, `user_profiles`, `addresses`, `contact_channels`, `orders`, `loyalty_accounts`, `email_log`

### S15. GDPR erasures (ongoing, EU)
- **What**: EU shoppers file deletion requests.
- **Mechanism**: `data_subject_requests` -> users get `deleted_at`, names and emails are
  cleared; orders remain with NULL `customer_email`; some downstream tables keep orphaned
  history.
- **Overview signal**: small, growing gaps in customer-level reports for EU merchants.
- **Drill-down**: orders without customer data -> erasure requests over time.
- **Tables**: `data_subject_requests`, `data_exports`, `consent_records`, `users`, `audit_log`, `access_log`, `legal_holds`, `retention_policies`, `audit_log_archive`, `compliance_events`, `unsubscribes`, `files`

## More stories

### S16. Credential-stuffing attack (2025-02-08 to 2025-02-11)
- **What**: a botnet replays leaked passwords against shopper logins of several merchants.
- **Mechanism**: a flood of failed `login_attempts` from few IP ranges; some accounts are
  taken over (new sessions, changed addresses, stored-credit spend); merchants respond with
  forced password resets, MFA rollout for staff, revoked API keys and tokens, stricter rate
  limits, rotated secrets; security staff file compliance events.
- **Overview signal**: a February spike in login failures and in "account access" tickets.
- **Drill-down**: failed logins by hour and IP -> accounts with a successful login after many
  failures -> their activity afterwards -> MFA adoption and resets.
- **Tables**: `login_attempts`, `sessions`, `password_resets`, `mfa_devices`, `oauth_accounts`, `api_keys`, `personal_access_tokens`, `service_accounts`, `rate_limits`, `secrets_vault`, `roles`, `permissions`, `role_permissions`, `user_roles`, `invitations`, `audit_log`, `access_log`, `compliance_events`, `email_log`, `sms_log`, `wallets`, `tickets`

### S17. Wrong VAT for Austria (2024-07-01 to 2024-09-30)
- **What**: after a tax-zone migration, one merchant charges German VAT (19%) instead of
  Austrian VAT (20%) on shipments to Austria.
- **Mechanism**: `tax_rates` / `tax_zones` / `postal_zones` mis-mapped for that merchant;
  `order_items.tax_total` too low for AT orders; fixed in October with corrective invoices
  and credit notes; the merchant's AT tax registration is updated. Its CHF orders to
  Switzerland share the faulty zone mapping, so the audit needs FX conversion to EUR.
- **Overview signal**: effective tax rate by country looks off for Austria in Q3 2024.
- **Drill-down**: effective tax rate by country and month -> by merchant -> by tax zone and
  postal code -> corrective credit notes.
- **Tables**: `tax_categories`, `tax_zones`, `tax_rates`, `postal_zones`, `regions`, `cities`, `tax_registrations`, `credit_notes`, `credit_note_lines`, `invoices`, `exchange_rates`, `currencies`, `orders`, `order_items`

### S18. Holiday gift guide (Nov 2024 - Jan 2025)
- **What**: a home-and-living merchant launches a "Gift Guide" collection with bundles and
  cross-sell recommendations, and sells many gift cards.
- **Mechanism**: collections, bundles and product relations drive larger baskets; gift cards
  bought in December are redeemed in January (and ~15% never are: breakage); refunds of gift
  purchases go to store credit (wallets); customer-service notes on orders about gift
  wrapping and delivery dates.
- **Overview signal**: that merchant's average order value peaks in December; January revenue
  is unusually strong but cash came in December.
- **Drill-down**: AOV by month -> bundle and collection sales -> gift card issuance vs
  redemption and outstanding balance.
- **Tables**: `collections`, `collection_products`, `product_bundles`, `bundle_items`, `product_relations`, `gift_cards`, `gift_card_transactions`, `wallets`, `order_notes`, `orders`, `order_items`

### S19. Catalog data quality hurts conversion (all of 2025)
- **What**: at a fast-growing apparel merchant, new products are onboarded sloppily.
- **Mechanism**: many products lack a primary image or the size attribute; some are filed in
  the wrong category after a category-tree restructuring (closure table rebuilt in March);
  tags are inconsistent; these products convert far worse (`product_viewed` events but few
  `add_to_cart`) and get more "wrong size" returns.
- **Overview signal**: the merchant's conversion rate declines although traffic grows.
- **Drill-down**: conversion by product -> products missing media or attributes -> category
  tree changes -> returns with reason "size".
- **Tables**: `brands`, `categories`, `category_closure`, `product_categories`, `attributes`, `attribute_values`, `variant_attribute_values`, `product_media`, `tags`, `product_tags`, `products`, `product_variants`, `events`, `order_returns`, `return_items`

### S20. B2B price war and pipeline slump (2025)
- **What**: a competitor undercuts an office-equipment wholesaler; the wholesaler answers
  with a new volume-tier price list and a reorganized sales pipeline.
- **Mechanism**: lead conversion drops, opportunities stall in late stages, win
  probabilities fall; a new price list with price rules (volume tiers) in June; a parent
  company consolidates purchasing for its subsidiaries (account relationships); sales reps
  work contact lists; one large account is won back.
- **Overview signal**: the wholesaler's bookings fall in H1 2025 and recover in H2 at lower
  margin.
- **Drill-down**: pipeline by stage and month -> win rates by rep -> prices before and after
  the new list -> account hierarchy of the big accounts.
- **Tables**: `leads`, `pipelines`, `opportunity_stages`, `opportunities`, `deals`, `deal_line_items`, `accounts`, `account_relationships`, `contacts`, `contact_emails`, `contact_lists`, `contact_list_members`, `activities`, `price_lists`, `prices`, `price_rules`, `price_rule_conditions`

### Additions to existing stories
- **S6**: the June win-back email is an A/B test of two subject lines.
  - **Tables**: `ab_tests`, `ab_variants`, `ab_assignments`
- **S9**: the remaining agents lean on the knowledge base (articles by category).
  - **Tables**: `knowledge_categories`

## Background (not stories, always on)

Seasonality and holidays, platform growth, Zipf-skewed merchants, products and shoppers,
currency and FX effects, ordinary refund / return / failure rates, ordinary churn.

## Implementation notes

All stories are switched on in `src/sim/stories.rs` (dates, merchants, effect sizes); the
rest of the simulator asks those functions, so effects propagate through the normal
mechanisms (queues, satisfaction, churn, renewals). Deviations from the text above:

- **S1**: Albion Parcel capacity 42% of normal (25% in Black Friday week); in-transit times
  rise from ~2.4 to ~13 days; Kennet Freight adds capacity for the switching merchant.
  Tracking events of congested parcels have status `exception`.
- **S2**: merchants with a second gateway retry on it (`gateway_timeout` on the first
  attempt, the payment row shows the fallback gateway, the payment method still says
  Halyard Pay); Halyard-only merchants lose most checkouts. Duplicate captures are refunded
  with reason `duplicate` and are not part of payouts or the ledger (netted by the gateway).
- **S3**: sessions bounce, checkouts fail, webhook deliveries fail and retry, storefront and
  checkout health checks are unhealthy (health checks are kept 35 days); the hourly
  abandoned-cart job (its runs are kept for the 10 largest merchants) fails with a 503 at
  14:00 and 15:00 and retries.
- **S4**: the supplier with the most volume among the European ones; quoted lead time (and
  `purchase_orders.expected_at`) stays at ~2 weeks, actual arrivals take 30 days in July
  and 45 from August; out-of-stock tickets; best sellers move to the second supplier on
  2025-10-06 (new supplier products not flagged preferred: master data lags).
- **S5**: the most popular stand mixer (featured in a spring campaign, so its appeal rises)
  has its lots in stock on 2025-03-03 and those received until April marked defective;
  ~1/3 of those units come back "Defective - stopped working"; recall adjustment on
  2025-05-19. At SF1 the merchant sells only a few mixers a month: the signal is clear from
  SF10 on.
- **S6**: a "Cancelled subscribers (spring 2025)" segment and a win-back email on
  2025-06-16 with a subject-line test; clicks restart subscriptions half of the time.
- **S7**: "Spring Mega Sale 2025" (40%, code MEGA40) and a creator campaign; the audience
  triples new signups, brackets sizes (up to three), regrets more, visits less, and
  unsubscribes three times as often; cohort "Spring Mega Sale 2025 - creator signups".
- **S8**: ring size ~30 accounts per SF unit (at least 25); 20 shared addresses (separate
  address rows with the same text), 4 shared devices (one device profile and anonymous id
  each), 6 cards (same brand and last4), 3 IP addresses; welcome coupon on every first
  order; 70% claim "Item not received" (refund reason `not_received` or `chargeback`);
  accounts suspended on 2025-10-27 with one compliance event.
- **S9**: the last two support agents hired leave on 2025-08-01 (fewer at small SF);
  support capacity 50% in Aug-Sep, 75% in October.
- **S10**: app installs on mobile visits after 2025-05-15 (push token, app user agent and
  `context.platform` in events, push notifications, 1.6x visit rate).
- **S11**: returning users get the treatment with p 0.68 (others 0.5); the treatment has no
  real effect.
- **S12**: SDK version 4.2.0 on mobile web 2025-06-02..10 sends every event twice (2-8 ms
  apart); 4.2.1 afterwards; `metrics_daily.events` doubles for those days.
- **S13**: 12% of the merchant's orders come through the ERP (audit rows with actor type
  `integration`); from 2024-10-27 their `placed_at`/`created_at` are local Berlin time.
- **S14**: the merchant brings ~1-3 years of customers when it joins (2024-02-05); the
  re-import on 2024-02-12 creates duplicates for 8% with email variants (plus-addressing,
  dots, capitalisation); half of their later orders are placed on the duplicate.
- **S16**: ~0.6 attempts per shopper and day for four days against three merchants, from
  three IP ranges with scripted user agents; 0.4% of attempts on real accounts succeed
  (session from the botnet, new address, "account access" ticket); forced resets, staff
  MFA, a login rate limit, a rotated API key and a compliance event on 2025-02-12.
- **S17**: Austrian orders in Q3 2024 are taxed at the German rate; the tax tables show the
  wrong rate for that quarter; an Austrian VAT registration from 2024-10-15. Swiss orders
  (exports, normally zero-rated) went through the same faulty zone and were charged the
  German rate too. They are priced in EUR (merchants sell in their own currency), so the FX
  part of the story does not apply.
- **S18**: "Gift Guide 2024" collection; gift orders twice as frequent with extra
  complementary items and more gift cards; gift-wrap notes; returns of gift orders bought
  Nov 2024 - Jan 2025 are refunded as store credit.
- **S19**: 45% of products created from 2025: 60% without images, no size attribute values,
  30% in another product type's category, fuzzier sizing; lower add-to-cart. On 2025-03-03 the
  category tree gets a "Clothing" level under Women and Men (leaves re-parented); the closure
  table is rebuilt, so all its rows of that merchant carry the rebuild time and the old
  top-to-leaf pairs now have depth 2. Products onboarded later still list only the old
  path in `product_categories` (no "Clothing" row).
- **S20**: fewer new accounts (60%) and a 35% win rate in H1 2025, some opportunities stall
  open; renewals 30% less likely; new price list "Volume pricing 2025" with deeper tiers from
  2025-06-02 (used at checkout from then); purchasing consolidated under one parent account
  on 2025-07-01; "Win-back 2025" contact list; the largest former account returns on
  2025-09-15.
