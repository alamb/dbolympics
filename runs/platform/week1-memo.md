**Memo: 2025 Platform Reliability Review**
To: Diego Morales (cc Samuel Whitcombe) — From: Lucas Moreau — Fri 2026-01-09

**1. Background jobs were essentially reliable all quarter — one isolated incident, not a trend.**
`job_runs` only retains Oct 1–Dec 31 2025 (confirmed by row counts, not an error — see caveat below), so I can't show a true year trend, but within that window: failure rate was 0.0% every single week except one, and queue delay/run duration were flat all quarter (~2.5s avg delay, ~5s avg duration, no drift). *Source: "2025 Q4 Weekly Job Reliability and Lateness."* "Late" jobs are not a problem — delay never moved.

**2. The one incident: the `carts.remind` job, week of Nov 24, hit exactly 10 tenants.**
All 20 failures in the quarter (0.09% of that handler's 21,867 runs) happened in a single week, in a single job type, affecting 10 of ~120 tenants — every other of the 8 job handlers had zero failures. *Source: "2025 Job Failures by Job Type (Handler)."* This is a contained, narrow incident, not a systemic job-reliability problem.

**3. It did reach top-tier accounts, not just the long tail.**
Of the 10 affected tenants, 6 had meaningful 2025 volume, including **Cragsley Outfitters — our #1 tenant by 2025 EUR revenue** — plus tenants ranked #4, #6, #7, #8. The other 4 affected tenants had zero 2025 orders (likely inactive/test accounts). *Source: "Tenant Size vs Carts.remind Job Failure Exposure."* I'm confident this wasn't volume-driven — small and large affected tenants had identical failure counts (2 each); it looks tied to a feature flag/config, not load.

**4. Monitoring caught it — barely, and maybe too late.**
Health-check unhealthy rate jumped to 2.44% the week of the incident vs. ~0.95–1.04% baseline in the surrounding weeks. *Source: "2025 Q4 Weekly Job Failures vs Health Check Alerts."* But ticket volume for the 10 affected tenants was already climbing 3–4 weeks before the job failures appeared in logs (1.0/tenant/week on Oct 27 → 5.8 by Nov 24). I'm **not confident** this is a clean "we caught it early" story — merchants may have felt it before our systems flagged it.

**5. Ticket-volume spike is real but mostly seasonal, not clearly caused by the incident.**
Affected tenants' ticket rate rose 174% from an Aug–Sep baseline (1.89→5.19/tenant/week); other tenants rose 148% over the same two periods (0.15→0.36). *Source: "Support Tickets: Affected vs Other Tenants."* Since both groups roughly doubled-to-tripled, most of this looks like a platform-wide holiday-season ticket bump. The extra ~26pp lift for affected tenants is suggestive but not strong evidence with only 10 tenants — I'd call this correlation, not proven cause.

**6. Webhook deliveries are stable overall but a chronic ~15% "never resolves" rate sits in ERP/WMS/Accounting sync integrations.**
Platform-wide success rate held 93–95% all 12 months, no trend. *Source: "2025 Webhook Delivery Reliability Trend."* But the worst 20 individual endpoints are dominated by ERP/WMS/Accounting sync webhooks stuck in pending/retrying 13–20% of the time, at real volume (e.g., Brennquell Kaffee's accounting sync: ~770 stuck deliveries). Several of these tenants (Brennquell Kaffee, Cragsley Outfitters, Ampfeld Elektronik, Marrowby & Vane) also appear in the carts.remind-affected list. *Source: "2025 Webhook Delivery Reliability by Endpoint."* This looks like a bug in our own retry logic for sync job classes, not a third-party provider problem.

**Caveats:**
- `job_runs` and `system_health_checks` appear to be retained only ~90 and ~60 days respectively — a full-year trend isn't possible from these tables as-is.
- My last request (webhook reliability, affected vs. other tenants) used an inconsistent tenant-matching definition and returned no "Affected" row — **don't cite that comparison**; the ERP/WMS overlap above is from eyeballing, not a validated cross-tab.
- Rate limiting (#4) is **not answered**: the warehouse has configured limits (`rate_limits`) but no log of actual throttle/429 events, so this can't be assessed from current data.

**Next steps:** (1) re-run the webhook-clustering comparison with the correct tenant join; (2) root-cause the ERP/WMS/Accounting sync stuck-retry rate — broad, fixable, affects many merchants; (3) push for longer retention on `job_runs`/`system_health_checks` so 2026's review isn't windowed to a quarter; (4) instrument rate-limit hit/block events if we want to answer Diego's #4 next year.