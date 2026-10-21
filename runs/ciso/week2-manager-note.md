Subject: Re: Account security snapshot — good start, three gaps worry me more now

Noah,

This is a strong first week — thank you. The sign-in and access-governance findings give me confidence I can defend in a questionnaire. Three things genuinely surprised me and I want to understand them better before Friday's memo, plus one follow-up.

1. **The audit trail gap is bigger than I initially feared.** If we can't show who deleted something and when for one of our most common objects, I have to assume the same is true for the things a security reviewer or a merchant's own auditor would actually ask about — customer records, orders, discounts, financial documents. Can you check whether this is a product-wide blind spot or specific to a few areas? I need to know how wide the hole is before I can tell leadership whether this is "fix one thing" or "fix how deletion logging works everywhere."

2. **Shopper MFA at zero is either a non-issue or a real liability — I need to know which.** Before this goes anywhere near a customer-facing security questionnaire, I want to understand what actually protects a shopper's account today if MFA genuinely isn't offered to them — is there anything else standing in for it (unusual-device checks, step-up verification at checkout, that sort of thing)? I don't want to either overstate a gap that doesn't matter or understate one that does.

3. **The churn-deactivation problem — how far does it go?** You found it through privileged roles, but I suspect it's broader: if accounts aren't deprovisioned when a tenant churns, that likely applies to their sessions, their API access, everything tied to that tenant, not just role holders. I'd like a sense of the overall scale of "access that should have been turned off but wasn't" so I can size the fix properly.

4. **On the old API keys — are any of them doing something sensitive?** Age alone isn't the risk; age combined with broad access is. Before I recommend a rotation mandate, I'd like to know whether those oldest keys are concentrated among a few tenants or integrations, and whether any of them look like they carry elevated access rather than routine, narrow use.

If we can only chase a couple of these deeply this week, I'd prioritize the deletion-audit scope (#1) and the churn-deactivation scope (#3) — those are the two a due-diligence team is most likely to ask us to prove, one way or the other.

Thanks,
Henrik