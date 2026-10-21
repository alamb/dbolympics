**Memo: Helpdesk Operating Review — Week 2 Findings**
**To:** Mia Schultz | **From:** Ben Carter | **Date:** Friday, Week 2

**1. Neither flagged merchant needs more agents — both breach spikes trace to delivery events, not staffing (high confidence).**
With the carrier-incident months stripped out, Marrowby & Vane's resolution breach rate drops to 22.5% (vs. Cragsley's 29.8% baseline) and Studio Vrij's drops to 7.0% — both *better* than our best-performing large merchant (Night 4, Report 3). That settles the staffing-vs-complexity question: this isn't an under-resourced team, it's two distinct, time-boxed delivery problems.

- **Marrowby & Vane:** 51.6% of their shipments go through Albion Parcel (Night 3, Report 1) — the same carrier the COO team flagged as broken platform-wide. Its on-time-within-3-days rate at this merchant held at 75–94% through October, then collapsed to 36.5% in November with average delivery time jumping from ~2.2 to 5.2 days (Night 4, Report 1) — and matching support tickets spiked from ~900 minutes/ticket to 9,533 (Nov) and 19,274 (Dec) average resolution time, 62–100% breach (Night 3, Report 2). **This is live and getting worse, not recovering** — there's no December row of delivered Albion shipments at all, consistent with parcels still stuck in transit at year-end. **Recommendation: escalate to carrier/ops, not headcount.**
- **Studio Vrij:** resolution time for delivery-tagged tickets spiked to 8,528–29,549 minutes in May–July 2025, then fully normalized by September (Night 3, Report 2) — almost certainly the same incident the VP CX team described last week as a "local May 2025 delivery breakdown... not platform-wide." I could *not*, however, pin this to their two main carriers: Lowlands Express and Polder Post both stayed in a normal 60–89% on-time band through that window (Night 4, Report 2) — so the mechanism is still open (possibly shipments that never got a `delivered_at` stamp at all, which our carrier averages wouldn't catch). **Whatever it was, it's over and fully resolved — no action needed, and it should be dropped from any staffing conversation.**

**2. Macro usage_count is not trustworthy — recommend retiring it from reporting.**
All five of our highest-usage macros show zero corresponding entries in `change_history` (Night 3, Report 2) — there is no audit trail behind that counter at all. I can't tell you what it's actually counting, and would rather say that plainly than repeat a number we can't defend.

**3. Topic/tag breakdown from last week is not biased.**
100% of 2025 tickets carry at least one tag (Night 3, Report 3) — no hidden untagged population changing the "damaged"/"account" slow-topic story.

**4. Knowledge-base deflection: can't be measured with current instrumentation.**
The `events` table has only 8 distinct event types platform-wide in 2025 (product views, search, cart, login, checkout, signup, etc.) — none track article/help views (Night 4, Report 3), and last week's page-path guess returned zero matches too. There's no usable signal for "did viewing an article prevent a ticket." This needs an instrumentation fix before we can answer Daniel's question, not another query.

**Confidence:** High on the Marrowby & Vane/Studio Vrij diagnosis and the "don't hire" conclusion — it's backed by three converging cuts of data. High on macro and tag-coverage findings (clean negative/positive results). Low/no confidence on Studio Vrij's root cause mechanism — flagging as open.

**Caveats:** Carrier on-time metrics only cover shipments with a `delivered_at` timestamp, which could understate problems where parcels are simply never marked delivered — worth keeping in mind generally, not just for Studio Vrij.

**Next steps:** (1) Flag the ongoing Albion Parcel/Marrowby & Vane situation to Merchant Success and Ops this week — it's worsening. (2) Ask engineering what `macros.usage_count` actually increments on before we use it again. (3) Propose instrumenting a `kb_article_viewed` event for next quarter's content-investment case.