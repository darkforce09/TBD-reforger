#!/usr/bin/env bash
# Drive enfusion-mcp over JSON-RPC from the shell, since the MCP tools are not
# registered as callable tools in this Claude session.
# Usage: mcp-call.sh <tool_name> '<json-args>'
set -euo pipefail
TOOL="$1"
ARGS="${2:-{}}"
# enfusion-mcp expects .pak files directly under <gamePath>/addons/, but this
# install nests them in addons/data/ and addons/core/. mcp-root is a symlink farm
# that flattens them (rebuild via scripts/setup-mcp-game-root.sh).
export ENFUSION_GAME_PATH="${ENFUSION_GAME_PATH:-/home/Samuel/.cache/enfusion-mcp-root}"
{
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"cc","version":"1.0"}}}'
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$TOOL" "$ARGS"
} | timeout 90 npx -y enfusion-mcp 2>/dev/null | python3 -c "
import sys,json
for line in sys.stdin:
    line=line.strip()
    if not line: continue
    try: o=json.loads(line)
    except: continue
    if o.get('id')==2:
        r=o.get('result',{})
        if 'content' in r:
            for c in r['content']:
                if c.get('type')=='text': print(c['text'])
        else:
            print(json.dumps(r,indent=2))
"
