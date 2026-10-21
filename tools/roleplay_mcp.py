#!/usr/bin/env python3
"""The engineers' warehouse tools for the role play (tools/roleplay.py), as a stdio MCP server.

One server per engineer request, attached by the CLI backends (Claude Code, Codex). The tools
themselves live in tools/roleplay_tools.py. Standard library only; newline-delimited JSON-RPC
2.0 on stdin/stdout.
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from roleplay_tools import TOOLS, WarehouseTools  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--db", required=True)
    ap.add_argument("--dry-log", required=True)
    ap.add_argument("--title", required=True)
    ap.add_argument("--question", required=True)
    ap.add_argument("--tags", required=True)
    ap.add_argument("--state", required=True, help="file the submitted query is written to")
    a = ap.parse_args()
    tools = WarehouseTools(a.bin, a.db, a.dry_log, a.title, a.question, json.loads(a.tags), a.state)

    def reply(id_, result=None, error=None):
        msg = {"jsonrpc": "2.0", "id": id_}
        if error:
            msg["error"] = error
        else:
            msg["result"] = result
        sys.stdout.write(json.dumps(msg) + "\n")
        sys.stdout.flush()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        msg = json.loads(line)
        method, id_ = msg.get("method"), msg.get("id")
        if id_ is None:
            continue  # notifications
        params = msg.get("params") or {}
        if method == "initialize":
            reply(id_, {"protocolVersion": params.get("protocolVersion", "2025-06-18"), "capabilities": {"tools": {}},
                        "serverInfo": {"name": "warehouse", "version": "1"}})
        elif method == "ping":
            reply(id_, {})
        elif method == "tools/list":
            reply(id_, {"tools": [{"name": t["name"], "description": t["description"], "inputSchema": t["schema"]} for t in TOOLS]})
        elif method == "tools/call":
            text, err = tools.call(params.get("name"), params.get("arguments") or {})
            reply(id_, {"content": [{"type": "text", "text": text}], "isError": err})
        else:
            reply(id_, error={"code": -32601, "message": f"method not found: {method}"})


if __name__ == "__main__":
    main()
