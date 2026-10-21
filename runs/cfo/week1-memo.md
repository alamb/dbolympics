**To:** Tobias Lindqvist, CFO
**From:** Kofi Mensah
**Re:** 2025 revenue, merchant concentration, and refunds — findings from this week's reports

**1. 2025 revenue (processing fees only)**
*"Corrected Tidewick Revenue by Month, 2025"* — Tidewick's processing-fee revenue for 2025 was **€93,583** on **€3,972,988** gross settled volume (blended fee rate 2.4%), rising seasonally into Q4 (€14,266 fee revenue in December alone vs. €7,925 in January). I'm confident in these totals: they were cross-validated against an independent currency-level reconciliation and tie out exactly.
**Caveat — I cannot show the full picture Tobias asked for.** The warehouse has no table for Tidewick's own platform-plan billing of merchants; the `plans`/`subscriptions`/`invoices` tables here record each *merchant's* billing of *their* customers, not Tidewick billing the merchant. So tonight's number is the processing-fee half of revenue only — the plan-fee half will need to come from a different system (likely Tidewick's own finance/billing tool), which I'd flag as a follow-up for next week.
**Also flag:** the first draft of this report understated both gross volume and fee revenue by roughly half; I traced and fixed it by rebuilding with a validated per-payout EUR conversion method, confirmed against an independent currency reconciliation. The figures above are the corrected ones.

**2. Merchant concentration**
*"Top Merchants..."* and *"Merchant Concentration Trend"* — The top 5 merchants were 47.5% of 2025 gross volume (down from 52.5% in 2024); top 10 were 64.5% (down from 65.7%). Cragsley Outfitters alone is 17.7% of volume. The active merchant base grew from 92 to 113 over the year, which is diluting concentration slightly even as the top names stay dominant. **Confident** in these figures (two independently-built reports agree to the cent). Read: concentration risk is real (two-thirds of volume in 10 accounts) but easing, not worsening, as new merchants are added.

**3. Refunds (bonus — scope allowed it this week)**
Apparel & Footwear runs a structurally high refund rate (19–22% every quarter of 2025, essentially flat vs. 2024's 21–26%) — not a new problem, just how that vertical behaves. More interesting: the **company-wide refund rate rose through 2025** (7.8% → 10.7% → 13.5% → 11.9% by quarter), and hand-checking the numbers shows this wasn't Apparel — it was a **non-Apparel spike** (rate jumped from ~2% baseline to 9.1% in Q3). Drilling down, this traces to a handful of smaller merchants, not a vertical-wide trend: Office Supplies B2B jumped to 14.8% in Q3 driven by OfficeHub Nordic (21%); Sports & Outdoor ran persistently elevated (10–16%); a few tiny merchants (Fell & Fjell, Trailmark) spiked to 25–39% in single quarters on thin volume.

**Caveat — treat the refund quarter-by-quarter numbers with caution.** I cross-checked `payments.amount_refunded` against the independent `order_returns` ledger and they disagree by 16–40% per quarter, with no consistent direction (payments-based figure is both higher and lower than returns-based figure in different quarters). This is most likely a timing lag (a purchase captured in one quarter can be refunded in a later one), but I haven't proven that yet, so I'd hold off treating the exact Q3 spike magnitude as gospel — the *direction* (a real, merchant-driven non-Apparel increase) I trust more than the precise size.

**Next steps**
- Reconcile `payments` vs `order_returns` by linking refunds to their originating order/purchase quarter rather than comparing calendar quarters in isolation.
- Get eyes on OfficeHub Nordic and the Sports & Outdoor merchants to understand if Q3's refund spike was a one-off (e.g., a product/quality issue) or ongoing.
- Follow up on FX exposure (#4) and B2B receivables aging (#5) next week as planned.