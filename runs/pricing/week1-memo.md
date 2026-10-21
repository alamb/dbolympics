# Memo: Promotion & Coupon Cost Review
**To:** Marco Bianchi | **From:** Oliver Grant | **Date:** Friday, 2026-01-05

## Findings

**1. The single biggest "discount" line isn't a promotion at all.** "Subscription delivery (prepaid)" tops every cost ranking (up to €24.3k on one tenant instance, Report N1-R1) but sits on €0 order revenue — these are 100%-comped orders (subtotal = discount, shipping/tax = 0, confirmed in N2-R1). This should be pulled out of the promo/coupon review entirely; it's a subscription-billing mechanic, not a growth lever.

**2. But it's leaking money.** 25-46% of these "free" orders per tenant have no overlapping active subscription (N3-R1). Drilling into who these customers are (N4-R1): **82% had a *canceled* subscription, averaging 117 days before the free order** — €57.2k in discounts given to lapsed customers. The remaining 18% show "active" status but didn't date-overlap my window (€11.7k) — this needs a technical check (my overlap logic, or a real timing gap) before I'd call it confirmed leakage. **This is the clearest actionable finding this week.**

**3. Real promotional discount cost is heavily concentrated.** Five tenants account for ~51% of all 2025 discount spend (Castell Fasteners 15.9%, Grind Collective 14.7%, Honeycomb Larder 9.9%, Cragsley Outfitters 8.8%, OfficeHub Nordic 8.6% — N1-R2). Two are B2B wholesalers running volume-tier discounts (structural, not marketing), two are subscription-box merchants (tied to finding #1-2).

**4. Incrementality: positive, but treat with real caution.** My first lift pass (post-discount total) was mechanically confounded; a corrected pre-discount-subtotal pass at the per-tenant-instance grain (N3-R2) looked uniformly *negative* — but that was an artifact of printing only the 50 worst of 219 small-sample rows. Re-aggregated order-weighted by promotion family (N4-R2), all four are positive: Welcome10 +107%, January Sale +69%, Black Friday +34%, Summer Sale +31%. **I'm confident spend is genuinely higher on discounted orders at this scale. I'm not confident it's causal** — bigger carts may simply be more likely to clear a coupon's minimum spend or get a volume tier, rather than the discount inducing extra spend.

**5. No coupon has a usage or per-customer cap configured anywhere in the warehouse** (N2-R2, zero rows matched). What looked like "repeat redemption abuse" in week one (customers using Black Friday/Welcome10 codes 2-5x) isn't a breach of any rule — there simply is no rule. This is a control gap, not proven fraud.

**6. Margin erosion beyond the discount line is real but concentrated differently by discount.** Welcome10 and January Sale instances run 2-10pp higher return rates than baseline, and two specific instances refund *more* than their own discount cost (Volume tier 10+: 162% of discount; Welcome10: 189% — N3-R3). Checking concentration (N4-R3): Volume tier 10+'s €18k is **top-3 refunds = 94%** of the total (one tenant, likely large B2B order cancellations — an outlier, not a pattern). Welcome10's €12.2k is **broad-based** (92 refunds, top-3 only 18%, median €77) — a genuine, recurring leak worth capping or investigating for product/fit issues.

## Caveats
EUR conversions rely on daily exchange rates; small-sample discount instances (n<10) produce volatile % figures and shouldn't be read individually. The "active-but-orphan" 18% needs a data check before acting on it.

## Next steps
1. Confirm the "active-but-orphan" subscription timing issue (bug vs. real).
2. Decide policy on lapsed-subscriber giveaways (~€57k/yr exposure).
3. Introduce usage/per-customer caps on Welcome10 and Black Friday codes.
4. Investigate Welcome10's broad-based refund pattern (reason codes).
5. Exclude Volume tier 10+'s outlier tenant before any "cap this discount" decision.