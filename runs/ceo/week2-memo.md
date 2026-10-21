# Memo: Revenue reconciliation, refund root-cause, and unit-economics scoping

**To:** Maren Okafor (cc: Tobias Lindqvist) | **From:** Hannah Brooks | **Date:** 2026-01-16

## Findings

**1. Revenue reconciliation — not yet closed, but I have strong evidence.** My 2025 fee-revenue figure (€53,186) now reconciles three independent ways: merchant-level, corrected vertical-level, and a currency-by-currency breakdown that sums to the exact same total. Finance's €93,583 is almost exactly **1.76x mine — the same ratio as a join-fan-out bug I caught and fixed in my own pipeline in week 1**. That's strong circumstantial evidence their query has a similar structural bug, not a genuine methodology difference. **Confidence: high on my number, not yet proven what's wrong in theirs.** I'd like a joint query walkthrough with Tobias's team rather than more one-sided guessing — 30 minutes should settle it.

**2. Refunds: concentrated incidents, not a trend.** Confirmed Kofi's example directly: **OfficeHub Nordic's refund rate spiked from a 2.99% baseline to 20.98% in Q3 2025 (€23,268 on 45 orders), then fell to 0% in Q4** — a resolved one-off, not ongoing. Our two biggest Apparel merchants (Cragsley, Marrowby & Vane) actually *improved* in both Q3 and Q4. Apparel's ~20-26% refund rate is a stable category baseline, not new. **Confidence: high.** This is a few-accounts issue, not a board-level trend.

**3. Unit economics scoping: revenue side is solid, cost side has a real data-quality blocker.** Take rates are tight and sensible across merchants/verticals (1.4%–3.6%, no outliers) — I trust this. But shipping cost data is not yet usable for ranking: **Deskline Supplies shows a shipping-cost rate of 120% of its own gross payment volume**, and the whole Office Supplies B2B vertical shows a shipping margin of **-14,505%** (€567 collected from customers vs. €82,825 paid to carriers). Some of this gap is plausible for B2B (bulk freight often isn't itemized as "shipping" to the customer), but costs exceeding 100% of volume is not credible as-is — it needs an Ops/Finance check before it goes anywhere near the board. **I'm not comfortable producing a ranked profitability list until this is resolved.** Country-level cuts aren't feasible yet either — merchant/shopper country isn't a clean field in this warehouse; flagging as a scoping gap, not attempting to force it.

**4. Accounts to watch.** Two distinct signals, worth keeping separate:
- *Platform billing risk* (merchants behind on paying us): only 4 tenants are past_due/suspended/churned, all small (<€1.3k TTM fee revenue each) — not currently a concern.
- *Receivables/health risk* (merchants' **own** customers owing them money — a merchant cash-flow signal, not money owed to us directly): **Kontor24 Bürobedarf stands out with €116.7k unpaid** (59% of all outstanding B2B receivables, half in one €49.4k invoice) — bigger than the Castell Fasteners example by 8x. **Castell Fasteners confirmed**: €14.5k unpaid, 29 days overdue, traffic down 43%. Both are Industrial/Office Supplies B2B accounts.

## Caveats
- Shipping cost data, as flagged above, is not trustworthy for cost-to-serve work yet.
- `net_terms_days` was null for every merchant in the receivables report — we can't yet tell "slow relative to terms" from "just large."
- Refund amounts in the unit-economics view use `order_returns`, not `payment_refunds` (no clean FK links them) — close enough directionally, not exact.

## Next steps
1. Joint query walkthrough with Tobias's team on the revenue discrepancy — bring both queries side by side.
2. Get Ops/Finance to sanity-check Deskline Supplies' shipping cost data before any unit-economics number goes to the board.
3. Once shipping cost is trusted, extend the rate-based unit-economics view (already built, by vertical and top merchants) into the ranked month-end deliverable.
4. Scope what "country" should mean for the board ask before committing more slots to it.