# Memo: Cart-Leak Deep Dive
**To:** Jonas Feld, VP Product **From:** Emeka Obi **Date:** 2026-01-16

## Findings

**1. The real divide at the cart stage is login status, not basket size.** Guest/anonymous carts (never logged in) converted at **exactly 0.0%** in 2025 — both single-item (0.0% of 6,834) and multi-item (0.0% of 2,054) — while logged-in shoppers converted at 51.0% (single-item) to 67.8% (multi-item). *Report: "Single-item vs. multi-item conversion, controlling for guest vs. logged-in status."* **Confident** this split is real; the 0% is so clean it's almost certainly structural — our schema requires a user attached to place an order, so a cart tied to no account essentially cannot complete. This is a bigger lever than anything about basket contents.

**2. Within logged-in shoppers, basket size still matters.** Multi-item carts convert 17 points higher than single-item (67.8% vs. 51.0%), consistent with the price-tier and duration-bucket patterns from last week, now confirmed within the "can actually convert" population. **Confident.**

**3. I need to retract last week's "email nudge" framing — there is no cart-recovery email running today.** I dug into which emails actually landed in that 56%-conversion group, and they're **order confirmations, shipping/delivery confirmations, and refund notices** — i.e., emails that go out *because* someone already transacted, not reminders sent to stalled shoppers. *Report: "What kind of email reaches shoppers who recover a stalled cart?"* The correlation I flagged last week was concurrent purchase activity, not a nudge causing recovery. **This changes my answer to your Q3: no organic recovery mechanism currently exists to learn from — we'd be building one from scratch.**

**4. Consistent with #3, the Nov 24 reminder-email bug doesn't show up in the data.** A weekday-matched baseline (8 prior Mondays/Tuesdays for Cragsley) shows Nov 24 (36.7%) and Nov 25 (31.3%) sit comfortably inside normal week-to-week swings (21.4%–52.6%). *Report: "Weekday-matched cart conversion baseline."* Combined with last week's flat Jan–Oct monthly trend, I'm now fairly confident: **the cart bleed is a stable, structural pattern, not a recent regression** — which matches your framing of "a structural product problem" rather than something that broke.

## Confidence summary
- **High confidence:** guest-vs-logged-in is the dominant gate; basket size matters secondarily among logged-in shoppers; the bleed is structural, not worsening; no cart-recovery email exists today.
- **Open:** *why* shoppers don't log in (friction? trust? no account yet?) — I haven't looked at that directly.

## Caveats
- The guest 0% finding reads as a data-model fact, not a subtle behavioral signal — worth a quick confirm with Engineering on whether guest checkout is offered at all, or silently non-functional.
- Daily sample sizes around the Nov 24 week were small (35–55 carts/day); the weekday-matched comparison is the best read I can get without a controlled test.
- I no longer trust the "received email → 56% conversion" number as evidence of anything actionable — flagging so it doesn't make it into a deck.

## Next steps
1. **Find out why shoppers don't log in** — is it an extra step, a trust issue, or no incentive? This is now the highest-value question for the cart leak.
2. **Design and test an actual abandoned-cart email** targeted at logged-in, single-item stalled carts (headroom: 51% → up to 68%) — none exists today, so this needs building and a proper A/B, not just more querying.
3. Resume the vertical-controlled returns/delivery question and the causal retention question when a slot frees up, as agreed.