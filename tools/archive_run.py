#!/usr/bin/env python3
"""Archive a role-play run (tools/roleplay.py --out DIR) in runs/, redacted.

    python3 tools/archive_run.py tmp/roleplay/run6            # -> runs/

Every file of the run is copied, transcripts included: this is the record of what the agents
saw and did. Claude Code's transcripts also hold the context it injects, such as the account
e-mail address and the home folder of the person who ran the role play; these are replaced by
a visible marker (`[redacted: e-mail address]`, ...). Nothing else is changed.

runs/ holds one run: its files plus `ARCHIVE.json` (source, time, repo commit) next to
runs/README.md; it must hold nothing else before. The archive is verified after writing: every
file is read back and compared with its redacted original.
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Personal data of whoever ran the role play, which Claude Code puts into the transcripts
REDACT = [
    ("e-mail address", re.compile(r"(?<=email address is )[^\s@]+@[^\s@]+?(?=\.?\s|\.?\")")),
    ("user name", re.compile(r"(?<=/Users/)[^/\s\"]+|(?<=/home/)[^/\s\"]+")),
    ("per-user temporary folder", re.compile(r"(?<=/var/folders/)[a-z0-9_]{2}/[a-z0-9_]{20,}(?=/)")),
]


def redact(data, counts):
    try:
        text = data.decode()
    except UnicodeDecodeError:
        return data  # not text: copied as it is
    for what, pattern in REDACT:
        text, n = pattern.subn(f"[redacted: {what}]", text)
        counts[what] += n
    return text.encode()


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run", type=Path)
    a = ap.parse_args()
    src = a.run.resolve()
    dst = ROOT / "runs"
    if any(p.name != "README.md" for p in dst.iterdir()):
        sys.exit(f"{dst} already holds a run")
    files = sorted(p for p in src.rglob("*") if p.is_file())
    total = sum(p.stat().st_size for p in files)
    counts = {what: 0 for what, _ in REDACT}
    meta = {
        "source": str(src),
        "archived_at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "repo_commit_at_archiving": subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=ROOT).stdout.strip(),
        "files": len(files),
        "bytes": total,
    }
    for p in files:
        out = dst / p.relative_to(src)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(redact(p.read_bytes(), counts))
    (dst / "ARCHIVE.json").write_bytes(redact((json.dumps(meta, indent=1) + "\n").encode(), counts))
    # read everything back and compare with the redacted originals
    for p in files:
        if (dst / p.relative_to(src)).read_bytes() != redact(p.read_bytes(), {what: 0 for what, _ in REDACT}):
            sys.exit(f"verification failed for {p}; remove {dst}")
    print(f"archived {src} -> {dst.relative_to(ROOT)}: {len(files)} files, {total / 1e6:.1f} MB (verified); redacted: {counts}")


if __name__ == "__main__":
    main()
