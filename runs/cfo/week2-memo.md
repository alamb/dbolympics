**To:** Tobias Lindqvist, CFO
**From:** Kofi Mensah
**Re:** Audit readiness, refund reconciliation, and plan-fee status

**1. Audit readiness — the money trail holds up, with one open discrepancy and one bounded list to hand over.**
*"End-to-End Transaction Walkthrough"* (25-payment sample) and the follow-up ledger drill-downs confirm the core mechanics are correct: payment → refund → payout settlement ties out, and the double-entry bookkeeping behind it is textbook-correct (Sales revenue/VAT credited at capture, Refunds-and-returns/VAT debited and clearing-account credited on refund — confirmed line-by-line for 9 sample transactions). My first attempt at a scale check looked alarming (ledger revenue running 14-16% below payments net amount every quarter), but that was **my error**, not a data error: I was comparing tax-exclusive ledger revenue to tax-inclusive payment amounts. Once I added VAT back into the ledger side (*"Revenue Reconciliation... Including VAT"*), the gap shrinks to -2.0% to +0.5% per quarter — effectively a clean tie-out.
**Confident:** the accounting logic is sound and ~98-99% of 2025 payments (card, wallet, bank_transfer, credit) reconcile cleanly (*"Ledger Reconciliation Outcome by Payment Method"*).
**Open item for the auditors:** 54 bank_transfer payments have real, unexplained ledger mismatches (full list pulled, largest is €9,212 on OfficeHub Nordic order SO-100403) — small in count, worth individual review before the audit starts.
**Caveat I haven't resolved:** payments with *no ledger entries at all* were counted at 67 total earlier this week but at 100 (all in an "unknown payment method" bucket) in tonight's by-method breakdown — a 33-payment discrepancy between two of my own queries that I didn't have a slot left to chase down. Treat both numbers as directional, not final, until reconciled.

**2. Refund figures: the 16-40% gap between sources is mostly timing, not an error.**
*"Refund Attribution Lag"* shows 78.6% of 2025 refunds land in the same quarter as the original purchase, 20.0% in the quarter after, and only 1.5% have no matching purchase at all. Payments-based refund totals (bucketed by purchase date) and order_returns-based totals (bucketed by refund date) will therefore never match quarter-for-quarter by construction — both are "correct," they just answer different questions. **Recommendation:** use purchase-quarter attribution (payments table) for margin-by-cohort, refund-event-date (order_returns) for cash-flow timing, and say so explicitly if asked.

**3. Plan-fee revenue: still a genuine gap, not a query problem.**
I re-confirmed this warehouse has no table recording Tidewick's own platform-plan billing of merchants — everything here models merchants billing *their* customers. This can't be solved with a cleverer query; I'd recommend pulling it from the corporate billing/finance system for the board deck.

**4. Leakage sizing: small relative to materiality.**
Coupon reuse beyond a customer's first use totals only ~€1,014 across 2025's top 20 codes — immaterial. Combined with Pricing's free-delivery (~€57k) and Welcome10 over-refund (~€12.2k) findings, total known leakage is roughly €70k against ~€4M in 2025 volume — worth fixing (cap the coupons) but not an audit-scale concern.

**Next steps:** reconcile the 67-vs-100 no-ledger-entries discrepancy; have someone pull the 54 bank_transfer payments for manual review; and start scoping where plan-fee data actually lives so I'm not presenting half of 2025 revenue again next quarter.