#!/usr/bin/env python3
"""Shared JSON-RPC consumer for the enfusion-mcp call path (T-090.0).

Reads newline-delimited JSON-RPC from stdin (the MCP server's stdout) and prints the
`id==2` tool result text to stdout, then EXITS IMMEDIATELY — closing the pipe so the
upstream stdio server gets SIGPIPE and dies, instead of the caller waiting out the
timeout. Single source of result-extraction + exit-code logic, reused by the one-shot
path, the daemon path, and the offline self-test.

Exit codes (the mcp-call.sh wrapper maps these to its locked contract):
  0  success  — id==2 result printed (content[].text, else pretty result object)
  1  empty    — id==1 init seen but stream ended with no id==2
  2  init-fail — no valid id==1 init / no usable JSON-RPC stream
  3  rpc-error — id==2 carried an "error" object (printed to stderr)
"""
import json
import sys


def main() -> int:
    saw_init = False
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except (ValueError, TypeError):
            continue
        if not isinstance(obj, dict):
            continue

        rpc_id = obj.get("id")
        if rpc_id == 1:
            saw_init = True
            continue
        if rpc_id == 2:
            # JSON-RPC protocol-level error (malformed request, etc.).
            err = obj.get("error")
            if err:
                sys.stderr.write(json.dumps(err) + "\n")
                return 3
            result = obj.get("result", {})
            # MCP tool-level error: a normal result flagged isError:true with the message in content[].
            if isinstance(result, dict) and result.get("isError") is True:
                texts = [
                    c.get("text", "")
                    for c in result.get("content", [])
                    if isinstance(c, dict) and c.get("type") == "text"
                ]
                sys.stderr.write(("\n".join(texts) if texts else json.dumps(result)) + "\n")
                return 3
            printed = False
            if isinstance(result, dict) and "content" in result:
                for chunk in result.get("content", []):
                    if isinstance(chunk, dict) and chunk.get("type") == "text":
                        sys.stdout.write(chunk.get("text", "") + "\n")
                        printed = True
            if not printed:
                sys.stdout.write(json.dumps(result, indent=2) + "\n")
            sys.stdout.flush()
            return 0

    # Stream ended without an id==2 response.
    return 1 if saw_init else 2


if __name__ == "__main__":
    try:
        sys.exit(main())
    except BrokenPipeError:
        # Downstream closed early; nothing to report.
        sys.exit(0)
