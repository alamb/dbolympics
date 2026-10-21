# Role-play runs

The role-play run the benchmark comes from, `run6`, as written by `tools/roleplay.py`, archived
here with `tools/archive_run.py`. `ARCHIVE.json` records the source path, the archiving time and
the repository commit; `index.html` browses the run (written by `tools/roleplay_viewer.py`).
The catalog and the website name the files as in the run, `run6/<path>`.

The archive is the full record, transcripts included, with one change: the transcripts also held
the context Claude Code added to every session, the e-mail address and the home folder of the
person who ran the role play. These are replaced by a visible marker (`[redacted: e-mail address]`,
`[redacted: user name]`, `[redacted: per-user temporary folder]`). The agents never used them.

| Archive | Backend, model | Tracks | Weeks × nights | Seed | Reports | Time | Size |
|---|---|---|---|---|---|---|---|
| `run6` | claude sonnet | all | 2 × 4 | 5 | ok 409, failed 2 | 361 min | 172 MB |

Earlier trial runs (run1 to run5) are not archived; the benchmark uses none of their queries.
