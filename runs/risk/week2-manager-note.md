Subject: Re: Questionnaire memo — good work, a few threads to pull

Claire,

This is a strong memo, and the gaps you flagged are exactly the kind of thing I'd rather surface now than have a prospect's security team find later. A few things stand out and I want to understand them better before we finalize anything.

The February 2025 login spike is the one that worries me most. A 10x jump in failures with no corresponding rise in flagged suspicious activity is a real detection gap, not just a reporting curiosity — if that was a credential-stuffing attempt, I need to know whether it actually got anywhere.

Questions for this week:

1. **On the February 2025 spike** — did it cluster around a handful of accounts, merchants, or sources, or was it spread evenly? And did any of the accounts that failed repeatedly go on to log in successfully shortly after? That tells us whether this was noise or an actual near-miss.

2. **On the three unresolved rejection dates** — rather than retrying the same technical path, can we establish, using whatever reliable timestamp we do have, roughly when each decision was made relative to the request date? I don't need the exact figure if it's not retrievable — I need to know whether we were plausibly within the deadline or not, so we can decide how to word that answer.

3. **On flagged incidents (the one we deferred)** — now that we're another week in, I want this one: how many incidents have been flagged, how severe, how long they typically take to close, and whether anything has sat open a long time. This is likely to come up in the same breath as the login spike.

4. **On the tables with no retention policy** — before I escalate this as a gap, I want to know how exposed we actually are: roughly how far back does that data go today? If it's a few months, this is a minor cleanup; if it's three years, it's a bigger conversation with Rosa.

Keep the role/permission logging gap and the broken query as noted — no need to keep spending slots chasing those; I'll raise the first with engineering myself and we can note the second as a known limitation in the memo.

Thanks,
Aylin