# Memo: Business orientation ahead of board meeting

**To:** Maren Okafor | **From:** Hannah Brooks | **Date:** 2026-01-09

## 1. Revenue is heavily concentrated in a handful of merchants and one vertical
Trailing 12 months (TTM), **Cragsley Outfitters alone drives 41.9% of fee revenue** (€22.3k of €53.2k), and the **top 5 merchants drive 72%** (*Merchant revenue concentration*). At the vertical level, **Apparel & Footwear is 54.6% of all fee revenue** from just 12 merchants (*Revenue by vertical, corrected*). Concentration has been fairly stable over 8 quarters — top-5 share has ranged 67–80% with no clear trend (*Revenue concentration trend by quarter*). **Confidence: high** — the merchant and vertical totals now reconcile to the same €53,186 figure after I caught and fixed a vertical-report bug mid-week.

## 2. Growth is real and broad-based, driven by existing merchants, not new ones
Active merchant count grew from 76 to 111 over 2 years; order count roughly doubled (1,717 → 4,259/quarter) (*Merchant and order growth by quarter*). But in every quarter, **>99% of GMV comes from merchants that joined in a prior quarter** — new signups contribute almost nothing in their first quarter (*GMV growth by cohort*). **Confidence: high.** Watch item: average order value has declined steadily, €257 → €210, as order count has outpaced GMV growth.

## 3. Processing take rate is stable — but I can't answer the platform-fee question
Take rate has held in a narrow 2.5–2.8% band with no trend (*Processing take-rate trend*). However, **I don't believe the warehouse models Tidewick's monthly platform-plan billing of merchants at all** — the billing/subscriptions tables are scoped to merchants' own end-customer subscriptions, not Tidewick-to-merchant billing. I can report on processing-fee revenue confidently, but **cannot produce the platform-fee vs. processing-fee mix you asked for from this data.** Worth asking finance/billing systems directly.

## 4. Vertical health: Apparel & Footwear is huge but slow-growing; smaller verticals are growing fast
YoY fee revenue growth: Industrial Supplies +80%, Consumer Electronics +82%, Pet Supplies +62%, Sports & Outdoor +52% — all from a small base. **Apparel & Footwear (54% of revenue) grew only 12%** (*Vertical revenue growth*). Jewelry & Accessories shrank -33% and lost its only growing merchant. Support load and satisfaction look broadly even across verticals (CSAT 3.0–3.7, nothing alarming) (*Vertical health*). **Confidence: medium** — small verticals (1–2 merchants) make growth % noisy; don't over-read them individually.

## 5. No acute red flags, but two things worth watching
- **Refund rate has crept up:** 8.8–10.7% through 2024, now 12.7–13.7% in H2 2025, with no corresponding spike in payment failures (steady ~7%) (*Operational warning signs*). Worth a root-cause look — product quality, a specific merchant, or policy change.
- **Activity gap:** almost all registered tenants (105 of 107 active-status) have a settled payout in the last 12 months, so the "inactive tenant" concern I flagged mid-week turned out to be smaller than feared — only 2 active tenants have gone quiet (avg. 118 days tenure, likely still onboarding) (*Merchant status and payout activity*). **Confidence: high**, and reassuring.

## Caveats
- The **payment-failure rate (~7%) and refund rate (~11–14%)** come from a same-slot report I didn't independently stress-test the way I did the payout data — I'd want one more reconciliation pass before quoting these to the board.
- **No platform-plan revenue data exists** in the warehouse as far as I can tell — flagging so it isn't assumed to be covered.

## Next steps
1. Confirm with billing/finance where platform-plan revenue actually lives, to complete the fee-mix picture.
2. Root-cause the refund-rate increase (by vertical/merchant) before year-end numbers go to the board.
3. Revisit AOV decline — is it mix-shift (smaller verticals growing) or something merchant-specific?