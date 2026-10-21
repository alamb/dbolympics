Subject: Re: 2025 Platform Reliability Review — good start, a few things I need tightened up

Lucas,

Thanks — this is useful and more honest than I expected on the limits of what we can see. A few things stood out and need more work before Friday's memo goes to leadership.

First, the retention gap is itself a finding, not just a caveat. We're flying with 90 days of job history and 60 days of health-check history — that means we genuinely don't know how 2025 looked before October. I want that stated plainly and separately in the memo as a reliability/observability gap in its own right, not buried as a footnote under the carts.remind story.

Second, the ERP/WMS/Accounting webhook stuck-retry pattern is, to me, the most important and actionable thing in here — more than the one-week carts.remind incident. I want that properly validated and sized, not eyeballed, before we say anything about it to leadership.

Questions for this week:

1. Can you redo the webhook comparison correctly and tell me: how many merchants are affected by this stuck-retry pattern, and what's the revenue or order volume behind them? I want to know if this is a handful of our biggest accounts or a long tail.

2. Has the ~15% stuck-retry rate on those sync webhooks been happening all year, or is it new/recent? Even without full job history, is there any way to tell whether this is a longstanding bug we've lived with or something that regressed recently?

3. On carts.remind — you said it looks tied to a feature flag or config rather than load. Can we get more confidence on that? I want to know if this is a one-off we already fixed, or a latent risk that could recur with other job types.

4. Setting aside strict proof of causation, can you give me your best read on whether the ticket-volume climb for the 10 affected tenants in the weeks *before* the failures showed up in our logs is something merchants were telling us about directly (i.e., check if their tickets mention sync/reminders/missing carts) — even a rough read would tell us if our support channel was effectively an early warning system we ignored.

On rate limiting: agreed, drop it for this cycle — don't spend a slot trying to force an answer out of config data alone. Just flag it as a 2026 instrumentation ask, as you already have.

Priority for your remaining slots: #1 and #2 above. The webhook/sync issue is the one I can take into a fix-it conversation with engineering next week if the numbers hold up — I'd rather have that nailed down than more polish on the carts.remind narrative.

Diego