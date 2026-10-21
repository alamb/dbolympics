**To:** Samuel Whitcombe
**From:** Arjun Patel
**Re:** Castell explanation, churn task force findings, top-15 support read

**1. Castell's traffic drop is real, not seasonal.**
Among the 6 B2B wholesale peers, only Castell (-10.6%) and Deskline (-12.5%) had negative Q4-over-Q4 session growth; the other four grew (Ballycorran +16%, OfficeHub +86.5%, Midland +11.1%, Polder +85.7%). Castell itself swung from +51.6% YoY in 2024 to -10.6% this year — a reversal, not a shared industry pattern. I'd raise it directly in their QBR. *(Source: "Castell Fasteners traffic drop")*

**2. Merchant churn is rare and concentrated in small/new accounts — no hidden big-merchant churn.**
Only 9 merchants are formally churned, all tiny (1–11 orders in their final quarter, several with no prior-quarter history at all — these look like accounts that never gained traction, not declining businesses). Separately, **zero** merchants with 20+ lifetime orders have gone silent for 180+ days. Looking at the full order-gap distribution, even our longest gaps among established merchants are unremarkable (Castell: 17 days since last order; OfficeHub: 18 days) — normal cadence, not a warning sign. *(Sources: "Pre-churn warning signs," "Silent-churn merchants," "Merchants slowing down")*

**3. The two candidate shopper-level early-warning signals don't hold up well.**
- *Marketing engagement:* open rates are nearly identical for shoppers who later go quiet (35.9–36.0%) versus those who stay active (36.4–37.3%) — disengagement doesn't clearly precede lapsing. *(Chloé's question #1 — weak signal)*
- *Support friction:* first-response time before a shopper lapses (30.78 hrs) is barely different from what still-active shoppers experienced in the same recent window (29.62 hrs) — this looks like the platform-wide holiday slowdown everyone saw, not a lapse-specific pattern. *(Daniel's question #1 — weak signal, likely confounded by season)*

**4. Refund friction couldn't be tested — there's no "slow" group.**
I split refunds into fast (≤7 days) vs. slow (>7 days) to test Daniel's hypothesis. Result: 8,035 refunds, **all** fast (avg 0.04 days to process), zero in the slow bucket. Refund processing is essentially automated and instant across the board, so this isn't a usable friction axis as defined. *(Source: "Refund friction and what follows")*

**5. Support/CSAT for the top 15 (picked back up).**
Cragsley (our #1 merchant) gets 149 tickets/quarter (10.9 per 100 orders) with CSAT 2.91. Marrowby (#2) has the worst combination: 78.95-hr first response, 136.6-hr resolution, CSAT 2.68. Fennel & Flint is the bright spot — fast response (18.5 hrs) and the best CSAT in the group (3.9). This matches Mateo's company-wide finding that speed drives satisfaction, and flags Marrowby for support attention. *(Source: "Support experience for top 15")*

**Confidence**
High: Castell's anomaly (clean peer comparison) and the "churn is rare/small" finding (exhaustive, not sampled). Lower: the shopper-level null results — lapsed-group samples are small (264–283 for marketing, ~1,548 for support) and I haven't controlled for calendar month, so a genuine but subtle signal could be masked by the holiday effect.

**Caveats**
- CSAT scores across the board are low (2.3–4.0); not specific to any one finding but worth flagging.
- Refund-friction test is inconclusive by construction, not a negative finding — don't report "refund speed doesn't matter" to the board, report "we couldn't test it with this data."
- "Lapsed" (180–365 days quiet) is my judgment call, not a validated churn definition.

**Next steps**
1. Re-test refund friction using return-request-to-refund lag (`order_returns`) instead of payment capture-to-refund, where more variance likely exists.
2. Re-run the support/marketing comparisons controlling for calendar month to separate "about to lapse" from "it was December."
3. For the board: so far traffic/session decline is the only early-warning signal I can show actually discriminates (Castell); recommend holding off on a marketing- or generic-support-based trigger until retested.