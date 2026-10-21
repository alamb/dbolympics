# Memo: Promotions, Coupons & the Subscription-Leakage Number
**To:** Marco Bianchi | **From:** Oliver Grant | **Date:** Friday (Week 2)

## Findings

**1. Incrementality holds up under a harder test.** Controlling for basket size (number of distinct items, not price), discounted orders still show higher value than non-discounted orders of the *same* item count — lift ranges from +13% (3-item orders) to +117% (1-item orders), positive in all 5 size buckets (*"Incrementality re-cut: order value matched by basket size"*). This survives the strongest control we've applied. **Confidence: moderate-high** — still correlational (heavier spenders may simply be more likely to use codes), but no longer explained away by "coupons just land on bigger carts."

**2. Welcome10's refund pattern looks like normal apparel behavior, not abuse.** Returns are spread across many categories (Outerwear and Boots largest, ~19% of refund value each; no runaway concentration), with 9-19 day order-to-return gaps — classic fit/try-and-return timing, not quick-flip behavior. Combined with last week's finding that this merchant's baseline return rate is already ~37% with Welcome10 only +2.3pp above it, the "refund exceeds discount cost" framing is mostly mechanical for any % discount at a high-return apparel merchant, not something the coupon is causing. **Confidence: high.**

**3. The 6-tenant leakage story is real but I can't give you a trustworthy total yet.** This is the important one. Tonight's attempt to finalize last week's ~€57k lapsed-subscriber number instead surfaced a scope problem: a clean, twice-independently-confirmed figure exists for the *original 6 tenants*, 2025 only — **461 orphan orders, €22,318.18** (reconciled exactly in two separate reports tonight and three weeks ago). But the same reconciliation run also surfaced **6 additional tenants running the identical discount that were never in any prior week's analysis** — led by one tenant alone (891 orphan orders, €36,731.77) worth more than all 6 known tenants combined. Meanwhile, the query meant to validate the full-history (~€57.7k) trend came back **empty — wrong definition entirely** — and a sub-analysis of the "active-but-orphan" bucket has now given three different answers this week (88, then 291, then 560 orders) depending on which engineer rebuilt it from scratch. **Confidence: low on any total above €22.3k. I will not pass the €57k figure, or any larger revised figure, to Tobias as confirmed.**

## Caveats
- Every rebuild this week that said "same population as the prior report" was reconstructed from a text description, not the actual SQL — and each rebuild drifted. That's the root cause of #3.
- The full-history query tonight used a logically broken orphan definition and should be discarded, not read as "zero leakage historically."
- The newly discovered tenants weren't missing by accident of small size — one is bigger than our entire known population, which suggests our original "top 50 by cost" view genuinely never surfaced them, possibly a tenant-filter or page-cutoff issue worth checking in the original query too.

## Next steps
1. Before anything goes to Tobias: get the original "orphan population" SQL preserved verbatim and reused (not re-described) for every follow-up query — this alone would have prevented this week's drift.
2. Re-run the full tenant list (not assumed to be 6) for "Subscription delivery (prepaid)," 2025 and full history, with that one locked query.
3. Re-run the active-bucket date classification once, carefully, and treat the 88/291/560 results as unusable until then.
4. Keep #1 and #2 (incrementality, Welcome10) as-is for this week's readout — those are solid.