**To:** Henrik Aalto, CISO
**From:** Noah Fischer, Analytics
**Re:** Account security snapshot — week 1 findings

Henrik, here's the evidence-based picture across your five questions, in priority order.

**1. Sign-in security: no attack signal, stable baseline.**
Weekly login attempts have run 1,300–2,400 with failure rates steady at 5–8% and ~1.5 attempts per distinct IP for 26 straight weeks (*Weekly sign-in attempt volume*). The one spike — +41% in the week of Nov 24 — I drilled into by tenant (*Login attempts by tenant, Nov 24 week*): it was spread across 110 merchants, top tenant's share (17.6%) matched its normal size, failure rates stayed in-band, and distinct IPs tracked distinct emails roughly 1:1. This reads as Black Friday/Cyber Monday shopping traffic, not credential stuffing. **Confident** in this conclusion.

**2. MFA: weak and flat, especially for shoppers.**
Staff MFA has plateaued at 54–57% all of 2025 (547 staff, 300 with confirmed MFA in Dec = 54.8%) — not improving. Shopper MFA is literally **0.0%** every month, across 11,207 shopper accounts (*Platform-wide MFA adoption trend*). **Confident** in the numbers; **open question** whether shopper MFA is an unused feature or simply doesn't exist for end customers — worth confirming with product before this goes into a questionnaire answer.

**3. Access governance: good news with one concrete gap.**
All 120 tenants carry two high-power system roles, Owner (26 permissions) and Administrator (25) (*Privileged role landscape*). I checked every account holding either role for dormancy: 16 qualify, and **all 16 sit on churned tenants** (9 merchants), still flagged `status = active`, up to 599 days since last login (*Dormant users holding Owner/Administrator access*). No dormant privileged access found on any live, paying tenant. The real issue isn't negligent review of active accounts — it's that **we have no process to deactivate accounts when a tenant churns.** Easy to name, easy to fix.

**4. API keys: well-used, but poorly rotated.**
No tenant shows an active key that's unused or abandoned (*API key hygiene*). But age tells a different story: 67.3% of active keys (76 of ~113) are 4+ years old, including one 2,743 days old (*Active API key age distribution*). There's no rotation field or process for api_keys in the schema — usage looks healthy, rotation hygiene does not.

**5. Audit trail: real, specific gap on deletions.**
Login/logout/create/export are logged for 100% of tenants, update for 94.2% (*Audit log coverage by action type*). But 'delete' events are thin — only 27 ever, 16/120 tenants. I tested this directly against real soft-deletions (*Audit-trail coverage of soft-deletions*): **615 products were soft-deleted across 91 tenants, and 0 of those show up as a 'delete' audit_log row.** Users fared better (27 of 39 logged, 69.2%). We could not credibly reconstruct "who deleted this product and when" today. Compliance monitoring is also thin: only 9 compliance_events total in 3 years (all resolved) — too sparse a sample to call it a proven control (*Compliance event resolution health*).

**Caveats on the data:**
- The users table's earliest soft-delete (2020-04-28) and the oldest API key (~2018) predate our 2023 warehouse history — likely carried-over legacy timestamps; worth a data-quality check before quoting ages externally.
- Campaigns/coupons/discounts/promotions show zero soft-deletions ever — I can't tell if that's a true audit gap or those features are simply never deleted in practice.

**Next steps I'd pursue:** (1) confirm whether shopper MFA is a real product gap or out of scope; (2) quantify the churn-deactivation gap across all entity types, not just role holders; (3) check whether non-product entities (orders, users, discounts) have the same delete-logging blind spot as products, since one table showing 0% is the most audit-questionnaire-relevant number we have.