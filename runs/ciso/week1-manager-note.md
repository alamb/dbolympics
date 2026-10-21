Subject: Where I'd like your help first — account security snapshot

Hi Noah,

Good to have you working with me this week. I'm still building my mental map of the business, so let's start broad and sharpen later. Here's what's on my mind, roughly in priority order:

1. **How exposed are we on sign-in security today?** I want to understand how merchant staff and shoppers are authenticating — are we seeing risky patterns like repeated failed logins, credential-stuffing-type behavior, or logins from unusual places? I need a feel for the baseline before I can say whether we have a problem.

2. **How strong is our multi-factor authentication posture?** I'd like to know roughly how many of our merchants' staff (and ideally shoppers, if meaningful) actually have MFA set up and active, versus how many are relying on password-only access. This matters because it's one of the first things enterprise security reviewers ask about, and I need to know whether we can answer confidently or whether it's a gap we need to close.

3. **Who holds powerful access, and is it still appropriate?** I'm interested in how permissions and roles are granted and whether there's good hygiene around it — e.g., are there dormant privileged accounts, or grants that nobody seems to be reviewing. This feeds directly into access-control questions in security audits.

4. **How healthy are our API keys and service accounts?** These are often the weakest link — long-lived tokens that nobody rotates. I'd like a sense of how many are active, how old they are, and whether any look abandoned or over-privileged.

5. **Can we actually reconstruct "who did what, when" if asked?** I want to understand how complete and reliable our audit trail is in practice — are sensitive actions consistently logged, and would we be able to produce a credible record if a merchant or regulator asked us to investigate an incident?

What I need this for: I'm walking into enterprise deals where security questionnaires will ask us point-blank about authentication strength, access governance, credential hygiene, and auditability. I need an honest, evidence-based picture — good or bad — so I can brief the CEO and sales on what we can credibly claim today versus what needs fixing before the next review cycle.

Given the slot budget, let's pick whichever 3 of these will give us the most defensible starting picture this week, and we can go deeper on the rest next week.

Thanks,
Henrik