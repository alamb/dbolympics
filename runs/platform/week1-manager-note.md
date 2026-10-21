Subject: 2025 platform reliability review — what I need by Friday

Lucas,

Leadership wants a year-in-review on platform reliability for 2025, and I'm leading it jointly with Samuel Whitcombe (Merchant Success). The deliverable is a memo that tells leadership how reliable the platform actually was last year, where it hurt merchants most, and what that implies for our 2026 investment priorities (reliability engineering headcount, which integrations/carriers to fix or replace, whether we need stricter rate limiting or SLAs). Samuel's side will map our findings to specific merchant relationships and churn risk, so where possible, surface things in a way that can be tied back to individual merchants rather than only platform-wide averages.

Given you only get a handful of report slots a night, here's what matters most to me, roughly in priority order:

1. **How often did background work actually fail or run late in 2025, and is it getting better or worse?** I want a view across the year (not just a single average) so we can see trend, not just a snapshot.

2. **When something broke, which merchants bore the brunt of it?** I'd rather see a short list of the merchants hit hardest by job failures, slow jobs, or delivery problems than a platform-wide blended number — that's what Samuel needs to go have conversations.

3. **Were there periods where our own health checks told us something was wrong, and did that line up with the times merchants complained or things failed?** I want to know if our monitoring is actually catching problems early or if we're flying blind.

4. **Are merchants getting throttled by our rate limits in ways that look like it's hurting their business, rather than protecting us?** I want to know if this is a tuning problem or a capacity problem.

5. **How reliable are our webhook deliveries to merchants' own systems, and are there particular integrations or providers that are chronically troublesome?** This feeds a build-vs-fix-vs-drop decision on a couple of integration partners.

If you have to cut scope to fit your nightly slots, protect #1 and #2 — those are what the memo absolutely must answer. The rest can wait or be addressed at a higher level if time runs short.

Let me know if anything above is ambiguous before you spend a slot on it.

Diego