**To:** Samuel Whitcombe
**From:** Arjun Patel
**Re:** Merchant health ahead of January QBRs

**1. Biggest merchants, and their real trajectory**
Cragsley Outfitters (Apparel & Footwear) is clearly our largest account: €305k in Oct–Dec 2025 revenue, up **+19.8% YoY** on the same quarter last year (orders +10.8%). Marrowby & Vane is #2 (€115k, **+15.6% YoY**, orders +17.9%). OfficeHub Nordic (#3, B2B, €95k) is growing fastest among the top names at **+27.9% YoY** / +58.8% orders. Castell Fasteners (#4, B2B, €90k) is the laggard of the top four at only **+8.5% YoY** revenue despite +21.2% more orders — average order value is shrinking there. *(Source: "Top merchants: YoY holiday-quarter growth")*

Important correction to my own first cut: quarter-over-quarter numbers (Oct–Dec vs Jul–Oct) showed implausible swings — Cragsley +147%, Marrowby +144% — because Jul–Oct is a quiet pre-holiday period for apparel. The YoY comparison above is the trustworthy growth read; treat the QoQ figures as seasonal noise, not a trend.

**2. At-risk shortlist**
- **Kontor24 Bürobedarf** (Office Supplies B2B): €64,191 in overdue invoices — by far the largest AR exposure on the platform. Worth a collections conversation before anything else.
- **Castell Fasteners** (our #4 merchant): web sessions down **-44%** quarter-on-quarter and the weakest YoY growth of the top tier (+8.5%), plus €5,080 overdue. Softening traffic at a top account is the clearest warning sign I found.
- **Ballycorran Industrial Supply**: refund rate jumped from 0.1% to **11.1%**, sessions down -17%, €3,347 overdue.
*(Source: "Merchant risk signals")*

**3. Thriving merchants and the pattern**
Beyond the apparel leaders above, the standout pattern is **subscription-style repeat buying**: Food & Coffee merchants (Grind Collective 94.2% repeat rate, 10.5 orders/customer; Brennquell Kaffee 86.0%, 9.3 orders/customer; Honeycomb Larder 86.7%) and B2B wholesalers (Castell Fasteners 88.6%, OfficeHub Nordic 82.9%, Midland Safety Wear 84.8%) have much stickier customer bases than general retail (Cragsley/Marrowby sit around 58%). If there's a playbook, it's "recurring need + reorder cadence," not any size or region effect I could isolate. *(Source: "Merchant customer base health")*

**4. Customer base health (top merchants)**
Cragsley and Marrowby both convert over half their shoppers into repeat buyers (58.6% / 58.3%, ~2.6 orders/customer each) — healthy, not spectacular, and worth contrasting with the subscription merchants above when advising them on retention tactics.

**5. Support/CSAT — not covered this week**
I prioritized slots 1–4 and cut this one, per your note that I could trim the list. Recommend as next week's first request.

**Confidence**
High confidence in the YoY growth ranking and repeat-purchase rates (large, stable samples). Lower confidence in refund-rate figures for smaller merchants — several show mathematically impossible rates over 100% (e.g., Brick & Bobbin 291%, Kiezkind 160%), almost certainly refunds landing against payments captured outside the 90-day window. I'd exclude refund rate from anything client-facing until recomputed on a payment-cohort basis.

**Caveats**
- No country/region field exists on `tenants` in this warehouse, so I substituted industry/vertical for the "region" angle you asked about — a true geographic cut needs a different join I haven't tried yet (likely via `tax_registrations` or `addresses`).
- Plan tier (platform pricing plan) isn't modeled in this warehouse at all — can't speak to that dimension.

**Next steps**
1. Recompute refund rate on a payment-cohort basis (refund tied to the payment's own capture date, not the refund date).
2. Dig into Castell Fasteners' traffic drop specifically — is it seasonal for industrial B2B, or a real signal?
3. Pull the support/CSAT report for the top ~15 merchants.
4. Find a reliable country/region field for a true geographic cut.