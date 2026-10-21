**To:** Ines Laurent, DPO
**From:** Priyanka Desai
**Re:** Privacy & data handling — week 1 findings

**1. DSAR backlog is smaller and better-behaved than the first cut suggested.** Full history: 55 requests "completed" (all have `completed_at` set), 1 "received" and 1 "in_progress" (both from Dec 2025, well inside typical turnaround of 5–30 days — see monthly report), and 3 "rejected." The 3 rejected requests never get a `completed_at` stamp — a closure-tracking gap, not an active backlog, since "rejected" is a terminal decision. I confirmed by subject_user_id join that **all 3 rejected erasure requests exactly match 3 active legal holds** for chargeback disputes on the same user, placed 1 day after each request (Marrowby & Vane, Rue Lafayette Studio, Cragsley Outfitters). So: true open volume right now is 2 requests, both within SLA. **Confident** in these numbers; **median close time 3–29 days** across history looks healthy.

**2. Legal holds (3 total, all active) are the real story behind item 1 — and they look stale.** All three are payment-dispute holds, aged 289, 544, and 954 days (report: "Legal holds"). Chargeback disputes don't typically run 1.5–2.6 years. **I have not verified against payment/gateway data whether these disputes are still genuinely open** — that's the open question before we can say whether these should be released and the erasures fulfilled.

**3. Compliance events show no backlog, but volume is very low.** Only 6 incidents in 3 years, all resolved (0.1–29.2 days), severities only "medium"/"high." **Confident** there's no unresolved queue; **not confident** this reflects true incident volume vs. under-logging — I haven't cross-checked against audit_log/access_log independently.

**4. Consent: marketing_email opt-in genuinely eroded, then stabilized.** Opt-in rate fell from 99.4% (Jan 2023) to a low of 90.2% (Oct 2025), driven by withdrawal spikes in Jul–Aug 2025 (28 and 26 withdrawals vs. a ~10/month baseline), then ticked back to 90.5% by Dec 2025 as grants surged. **Caveat:** `analytics_cookies` shows a suspicious 100.0% opt-in with zero withdrawals across all 36 months — I don't yet trust this is real and haven't dug into why.

**5. Retention reality vs. policy — one clear failure, one partial gap.** Of 6 entity tables with policies: **email_log (730-day delete) is configured for all 120 tenants but `is_active=false` and has never run for any of them.** Live data check confirms it: 14 months of email_log data (Jan 2023–Jan 2024, ~44K+ emails) sit well past the 730-day line with nothing purged. **audit_log** looks healthier — most of 2023 is already cleared from the live table — but **Dec 2023/Jan 2024 (3,589 and 3,731 rows) are still present despite being past 730 days**, even though the job shows `last_run_at = 2025-12-31` and is active for most tenants. I haven't explained that gap yet. I also note `access_log`, `data_exports`, and `consent_records` have **no retention policy configured at all** — not a violation, but a governance gap worth flagging.

**What I'd look at next:**
- Get email_log purging actually turned on; this is the one unambiguous violation.
- Pull payment/dispute status for the 3 held accounts to decide if holds should release.
- Chase why the Dec 2023/Jan 2024 audit_log cohort wasn't archived (and check the 9 tenants flagged "never run" across several policies — looks like newly onboarded tenants, unconfirmed).
- Decide on retention rules for access_log, data_exports, consent_records.
- Sanity-check the analytics_cookies consent data and the fact that zero portability requests have ever been logged.

— Priyanka