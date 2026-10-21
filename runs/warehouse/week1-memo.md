**Memo: Inventory count accuracy — initial findings**
**To:** Tomasz Nowak | **From:** Fatima Zahra | **Date:** 2026-01-09

**1. There are two separate correction mechanisms — don't average them together.**
A routine, year-round stream (`stock_adjustments`, mirrored almost exactly in `stock_movements` kind=`cycle_count`: 10,194 vs 10,193 rows) posts **exactly 1 unit per event**, ~850/month, rising from 764 (Jan) to 946 (Dec) — a **24% increase over the year** (Report: *Monthly volume of stock_adjustments*). Net effect is shrinkage, growing from -256 to -396 units/month. Estimated value ≈ €260k for the year (~€20-25k/month).
Separately, a **406-event bulk batch, all on 2025-12-31**, sits in `stock_movements` kind=`adjustment` (Report: *Largest individual stock adjustments*) — net **positive** (stock found, not lost), ~€138k, reference_type='correction'. This reads like an annual stocktake true-up, not an ongoing process signal, and should not be blended into the monthly run-rate.

**2. Where it concentrates.**
The routine stream is **100% in "pick" locations** — 120 of 157 pick locations touched vs. **zero** of 157 bulk or returns locations (Report: *by storage-location kind*). No warehouse dominates: top 2 sites (Atlanta/Cragsley, Reading/Marrowby & Vane) = 18% of value, top 10 ≈ 52%, top 20 ≈ 76% — a long tail, not one or two sites.
The Dec-31 batch is almost entirely **B2B wholesale**: Deskline Supplies + OfficeHub Nordic alone = 60% of its value; the top 5 industrial/office-supplies tenants = ~94% (Report: *Which merchants drove the December correction*). Non-B2B tenants show trivial, near-zero amounts.

**3. Product vs. process.**
Discrepancies touch 105 categories but event volume concentrates in apparel/footwear (Tops 1,086, Trousers 885, Outerwear 742 events) — high-SKU, high-size-variant lines. Lot/serial-tracked categories (Food 100%, Appliances 76%, Grinders 65%) show *lower* normalized variance per unit sold than untracked apparel lines. **Reading this as a picking/handling issue in complex, untracked SKU ranges, not a lot/serial tracking weakness.**

**4. Reservations are reliable.** 99.3% of holds release on time; **0%** are stuck past expiry (Report: *Stock reservation outcomes*). Not a material overselling risk.

**5. Shrinkage vs. gain (Tobias's question).** Within the routine stream, reasons split roughly 48.5% "cycle count variance" (nets *positive*) vs. 51.5% explicit loss reasons — theft-suspected, damage, expired, each ~1,750-1,780 events, netting the entire -3,880 unit shrinkage. The loss categories are suspiciously evenly sized; I'd confirm this isn't a data artifact before sizing a provision off the 3-way split.

**Confidence:** High on the two-mechanism finding and the pick-location concentration (cross-checked across two independent tables, numbers reconcile exactly). Medium on category/velocity conclusions (thinner data, some normalization noise for low-volume categories like jewelry). Low on whether the Dec-31 batch is a real annual process or a data artifact — needs confirmation with ops before quoting to auditors.

**Caveats:** Values use proxy costing (preferred/cheapest supplier unit cost → EUR), not official inventory valuation — treat as directional, not audit-grade. Variants with no supplier cost on file are excluded from value totals (undercounts true exposure). Category rollups merge same-named categories across tenants.

**Next steps:** (1) Confirm with ops whether the Dec-31 batch is a scheduled annual B2B stocktake; (2) re-cost both streams using the official inventory valuation method; (3) drill into specific apparel SKUs/sizes driving pick-location volume.