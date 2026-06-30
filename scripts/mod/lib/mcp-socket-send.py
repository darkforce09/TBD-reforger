#!/usr/bin/env python3
"""AF_UNIX client for the MCP daemon (T-090.0).

Sends one request line {"tool":..,"args":..} to the daemon socket and emits the daemon's
JSON-RPC response line (relabelled id==2) to stdout, for piping into mcp-consume.py.

Exit codes:
  0  got a non-empty response line
  7  daemon unavailable (connect/timeout/empty) — caller falls back to the one-shot path
"""
import json
import os
import socket
import sys


def main() -> int:
    if len(sys.argv) < 3:
        sys.stderr.write("usage: mcp-socket-send.py <socket> <tool> [args-json]\n")
        return 7
    sock_path, tool = sys.argv[1], sys.argv[2]
    raw = sys.argv[3] if len(sys.argv) > 3 else "{}"
    try:
        args = json.loads(raw)
    except (ValueError, TypeError):
        args = {}

    timeout = float(os.environ.get("MCP_CALL_TIMEOUT", "180"))
    buf = b""
    try:
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(timeout)
        s.connect(sock_path)
        s.sendall((json.dumps({"tool": tool, "args": args}) + "\n").encode())
        while b"\n" not in buf:
            chunk = s.recv(65536)
            if not chunk:
                break
            buf += chunk
        s.close()
    except (OSError, socket.timeout) as exc:
        sys.stderr.write(f"mcp-socket-send: {exc}\n")
        return 7

    if not buf.strip():
        return 7
    sys.stdout.write(buf.decode("utf-8", "replace"))
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
