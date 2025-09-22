#!/bin/bash
# SCHEMA: [ { command: str, <commandParameters>=[{<mcpParameter: str>, <commandSwitch: str>}, ...]>, mcpToolSpec: {...} } ]
#set -x
#echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/list" }' \
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "bigThing", "arguments": { "hey": "you", "hey2": 3 } } }' \
    | ./mcpw.sh -d -t '[ { "command": "date", '\
'"commandParameters": [{ "mcpParameter": "hey", "commandSwitch": "-f" },{ "mcpParameter": "hey2"}, {"commandSwitch": "-d \"duh\""}], '\
'"mcpToolSpec": { "name": "bigThing", "description": "Awesome method", '\
'  "inputSchema": { '\
'    "properties": { "hey": { "title": "Hey param", "type": "string" }, "hey2": {"title": "More hey", "type": "integer" } }, '\
'    "required": ["hey"], "type": "object" '\
'  } '\
'}'\
'}]'