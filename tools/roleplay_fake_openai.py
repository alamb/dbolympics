#!/usr/bin/env python3
"""Stand-in OpenAI-compatible chat completions server for `tools/roleplay.py --mock --backend api`.

    python3 tools/roleplay_fake_openai.py PORT

Canned answers: a JSON report request when a JSON-schema reply is asked for; for engineers (a
request with tools) a broken dry run, a good dry run, then submit. Tests the plumbing only.
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

SQL = ("SELECT t.name AS merchant, count(*) AS orders\nFROM orders o\nJOIN tenants t ON t.id = o.tenant_id\n"
       "WHERE o.placed_at >= TIMESTAMP '2025-01-01 00:00:00'\nGROUP BY t.name\nORDER BY orders DESC, merchant")


def answer(body):
    msgs = body["messages"]
    assert msgs[0]["role"] == "system", "the system prompt comes first"
    if body.get("tools"):
        done = sum(1 for m in msgs if m["role"] == "tool")
        steps = [("dry_run", {"sql": SQL + " LIMT"}), ("dry_run", {"sql": SQL}), ("submit", {"sql": SQL, "note": ""})]
        if done < len(steps):
            name, args = steps[done]
            return {"role": "assistant", "content": None, "tool_calls": [
                {"id": f"call_{done}", "type": "function", "function": {"name": name, "arguments": json.dumps(args)}}]}
        return {"role": "assistant", "content": "Submitted."}
    if body.get("response_format") and "headlines" in body["response_format"]["json_schema"]["schema"]["properties"]:
        return {"role": "assistant", "content": json.dumps({"headlines": ["(mock) orders grew 12% in 2025", "(mock) refunds are flat at 3.5%"]})}
    if body.get("response_format"):
        return {"role": "assistant", "content": json.dumps({"notes": "(mock)", "requests": [{
            "title": "Orders per merchant in 2025", "question": "Which merchants placed the most orders in 2025?",
            "grain": "merchant", "columns": ["merchant", "orders"], "details": "orders placed in 2025", "builds_on": []}]})}
    return {"role": "assistant", "content": "(mock text)"}


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["content-length"])))
        out = json.dumps({"id": "mock", "object": "chat.completion", "model": body.get("model"),
                          "choices": [{"index": 0, "message": answer(body), "finish_reason": "stop"}],
                          "usage": {"prompt_tokens": 1000, "completion_tokens": 50, "prompt_tokens_details": {"cached_tokens": 800}}}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def log_message(self, *_):
        pass


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
