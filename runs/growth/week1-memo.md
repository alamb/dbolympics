**Memo: Growth Marketing — orientation week**
**To:** Chloé Marchetti **From:** Sofie Nilsen **Date:** Fri, 2026-01-09

Here's what four nights of reports tell us. I've ranked by what's most decision-relevant.

**1. Discounting needs a "promote more carefully" conversation — this is the headline.**
Platform-wide, discounted orders *looked* healthier than full-price ones (410€ vs 192€ AOV, fewer returns) — but that pooled number is misleading. Broken out by vertical, it reverses: in true consumer retail (apparel, beauty, electronics, home, sports), discounted orders have **equal-or-lower AOV and meaningfully higher return rates** than full-price orders (apparel: 39.0% vs 34.5% returned; electronics 17.4% vs 10.7%; sports 25.9% vs 19.6%). The original "healthy" signal was driven by two unrelated effects baked into the pooled average: B2B wholesalers where "discount" means contract pricing (44 items/order, 2,223€ AOV), and Food & Coffee/Pet subscription merchants running near-free trial offers (6-8€ AOV). **Confident**: the reversal is real and vertical-specific. Separately, the Jan/Jul spike in discount rate (53-59% vs ~35% baseline) wasn't a platform event — it traces to a handful of merchants (Marrowby & Vane, Cinder Peak, Ridgeback Gear) running sitewide sales those months.

**2. Traffic is concentrated, and "direct" dominance is a big-merchant story, not a tracking gap.**
57% of 2025 sessions are untagged "direct," up 26% YoY, but the top 2 merchants (Cragsley Outfitters, Marrowby & Vane) alone account for 31% of all platform direct sessions — this tracks with Tidewick's known long-tail distribution, not an analytics problem. Of the minority of orders marketing can claim credit for (attributions table, ~14% of all orders), paid_search dominates attributed revenue (78.6%, 2,503 orders, 256€ AOV) over email (18.7%). **Caveat:** attribution coverage is thin — most orders have no linked touchpoint, so I can't yet say what's really driving the other 86% of orders.

**3. Email is a quiet strength, but it's the only channel we actually use.**
106k sends in 2025, 99.5% delivered, 29.9% open rate, 2.4% click rate, bounce/unsubscribe near-zero — solidly in line with industry norms, not broken. But `campaigns.channel` only ever shows "email" — no SMS/push/social campaign activity is logged. Either merchants aren't using other channels, or they're not instrumented. Worth finding out which.

**4. Loyalty looks flat-to-negative; referrals look genuinely strong — don't treat them the same.**
Loyalty adoption is low (862 of ~6,200 active shoppers, 14%) and even within-industry, members don't out-order or out-spend non-members (e.g., apparel: 165€ vs 189€ AOV; electronics 446€ vs 1,114€). This isn't just a mix effect — I checked. Referrers, by contrast, order far more (8.0 orders/user vs 3.3 for everyone else) at a similar AOV, driving outsized revenue per head. **Open question:** is referring *causing* loyalty, or do already-loyal customers simply become referrers? I can't separate cause from selection with what we have.

**5. We do run A/B tests, but shallow ones.** 1,036 completed, only 4 running now, but average test duration is **3 days** and only 62% of completed tests have a recorded winner — that's a thin "acting on results" muscle, not a broken one.

**Caveats on the data itself:** money conversions silently drop rows missing an exchange rate for that day/currency — I haven't quantified how much that affects totals. `tenants.industry` is free text, not a controlled list, so vertical groupings are approximate.

**Next steps:** (1) quantify whether discount-driven returns actually erode margin, by tenant; (2) test referrer-then-order sequencing to check causality; (3) audit why no non-email campaign channel shows up; (4) look at why A/B tests close in ~3 days.