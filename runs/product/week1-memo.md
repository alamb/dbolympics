# Memo: Shopper Journey End-to-End
**To:** Jonas Feld, VP Product **From:** Emeka Obi **Date:** 2026-01-09

## 1. Storefront → order funnel
The biggest leak for our one-off retail merchants (apparel, beauty, electronics, pet) is **cart → order**, converting at only 27–61% (vs. 94–99% order → paid). *Report: "Storefront-to-order funnel, Q4 2025"; "Session-tracking coverage vs order volume."*

**Caveat:** for our 4 subscription/food-box merchants, "visits" is the wrong denominator — they have 0.5–0.7 visits per order, because recurring orders are system-generated renewals with no browsing session at all. I've excluded them from funnel conclusions; the funnel story below is retail-only. Confident on the segmentation; the underlying web-session instrumentation for these merchants may itself be a gap worth a separate look.

## 2. Checkout drop-off: a slow bleed, not a cliff
Once an order row exists, payment is nearly instant and near-universal (97.6% paid, median 0 hours — *"status-transition funnel, retail-only"*). The real leak is **upstream, in the cart**: only ~23–32% of carts resolve within an hour (and those convert ~100% — fast, successful checkouts). The other ~two-thirds linger 1 hour–7 days and convert at just 1–19%. This pattern holds consistently across all 6 retail merchants (*"Cart abandonment timing by tenant"*) — it's a product-wide behavior, not one merchant's cart-expiry setting. **Confident.** I could not reliably attribute this to device or acquisition channel — the cart-to-session join matched only ~1% of carts (most carts are guest/anonymous with no user_id to link on), so that cut is a schema dead end, not a null result.

## 5. Repeat purchase
40–62% of first-time buyers return within 90 days, with a 2023–2025 median of 44–78 days to the second order (*"Repeat purchase rate by cohort month"*). No anomalies across 34 cohort-months; **confident** this is a stable baseline. Given retention looks reasonably healthy, I'd weight the roadmap toward fixing the cart-stage leak (bigger, cheaper volume) over acquisition-funnel work.

## 3. Delivery and trust — weaker signal than expected
At the merchant level, higher late-delivery rates do **not** track with higher returns — if anything, apparel merchants (highest returns, 31–34%) had the *lowest* late-delivery rates, swamped by vertical (apparel returns are structurally high regardless of delivery speed). Re-testing **within** apparel only (*"Late delivery and returns, apparel"*) is inconclusive: Marrowby & Vane and Studio Vrij show late shipments with higher return rates (43.8% and 66.7% vs ~33–36% on-time), but Cragsley Outfitters shows the opposite, and the "late" buckets are tiny (3–73 shipments) — too thin to trust. Similarly, late September deliveries showed no meaningful drop in repeat purchase (58.9% on-time vs 54.5% late, n=11 late customers) — **not a real test**, just an underpowered one. **Open question, not answered this week.**

## 4. Does a return kill the relationship?
A modest effect, not a cliff: customers with a returned order repeat at 49.7% within 90 days vs. 57.7% for no-return orders, and take slightly longer (33 vs 28 median days) (*"Repeat purchase after a return"*). **Confident there's a real gap (~8pp); not confident it's causal** — returners may skew toward categories (apparel) with naturally lower repeat rates regardless of the return itself.

## Next steps
1. **Prioritize the cart-stage leak** — the 1-hour-to-7-day bleed is the single largest, most consistent volume opportunity (cart recovery nudges, faster checkout).
2. Re-run the delivery/returns/repeat questions with a full quarter of late shipments (current samples are too small) and control for vertical before drawing causal conclusions.
3. Investigate the subscription merchants' session-tracking gap — it may be masking real product-analytics blind spots beyond this report.