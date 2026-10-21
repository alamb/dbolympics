**To:** Henrik Aalto, CISO
**From:** Noah Fischer, Analytics
**Re:** Deletion-audit scope, churn deprovisioning, shopper MFA, old keys — answers

Henrik — here's what this week's follow-ups found, in the order you asked.

**#1 Deletion-audit gap: narrower than feared, but two real holes.**
A full census of every soft-deletable table (30 tables) shows only two have ever actually had a deletion: products (615 rows, 91 merchants, still 0% logged in audit_log) and users (39 rows, 69.2% logged). The other 28 show zero deletions ever — that's not evidence they're fine, just untested.
Orders are better news than last week suggested: the purpose-built `order_status_history` table shows **100% coverage** of every cancellation, and every status change, across 90–118 merchants — we can reliably show *when* an order changed state. But *who* cannot be shown: across 150k+ transitions, every status, every merchant, the actor field is populated **0.0% of the time**, with no exceptions. That's a systemic property of the table, not a cancellation-specific bug.
A smaller, confirmed gap: invoices are never formally "voided" (0 ever), but 85 "uncollectible" write-offs across 11 merchants have zero audit trail and no equivalent history table standing in. **Confident** in all four numbers.

**#3 Churn deprovisioning: real, but the live exposure is narrow.**
Every user on all 9 churned tenants is still `status = active` — universal, not limited to Owner/Admin as first reported. But exploitable risk is small: all "never-revoked" sessions on every churned tenant have already naturally expired (0 still valid). Two tenants (Dewdrop Studio, Aurum & Ash) still run one live API key each, both carrying `admin_api` scope and used in the last 5 days via what looks like an automated nightly ERP sync — a genuinely live, broadly-scoped credential still talking to a merchant who no longer pays us. **Confident**: the fix is a churn trigger that disables accounts and revokes keys/webhooks; those two tenants need attention now.

**#2 Shopper MFA: a real gap, not overstated.**
88.3% of shoppers (9,910) are password-only with no backup of any kind; only 11.7% get indirect protection via an OAuth login, and even that group has 0% confirmed MFA. Recent-activity rates are nearly identical across both groups (47.9% vs 46.9%), so this isn't just inactive accounts inflating one side. **Confident** this is a genuine liability.

**#4 Old API keys: broad access is the norm, not an age-driven drift.**
53 of 76 long-lived (4+ year) active keys (69.7%, 25 merchants) carry `admin_api` scope. Scope breaks down similarly across every age bucket (70–100%), so broad access isn't creeping in with age — it's simply how integration keys are provisioned platform-wide. The real risk is that these broad keys never rotate, and they concentrate on our biggest accounts: Cragsley Outfitters and Marrowby & Vane each run 4–5 of them.

**Caveats:** The 28 "zero deletions ever" tables prove absence of data, not absence of risk — don't read them as clean. Every key's "5 days since last use" timestamp clusters within minutes of each other on Dec 31, which smells like one nightly batch job rather than genuinely independent usage — worth confirming with engineering before treating usage as fully reassuring.

**Next steps:** (1) apply the churn-trigger fix immediately to the 2 live-key tenants, then design it platform-wide; (2) decide whether `order_status_history` needs an actor column — a one-table fix with outsized audit value; (3) scope a rotation mandate specifically for `admin_api`-scoped keys, starting with Cragsley and Marrowby.