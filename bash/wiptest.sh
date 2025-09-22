#!/bin/bash
# SCHEMA: [ { command: str, <commandParameters>=[{<mcpParameter: str>, <commandSwitch: str>}, ...]>, mcpToolSpec: {...} } ]
#set -x
#echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/list" }' \
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "bigThing", "arguments": { "dateParams": "-u" } } }' \
    | ./mcpw.sh -d -t '[ { "command": "date", '\
'"commandParameters": [{ "mcpParameter": "dateParams" }], '\
'"mcpToolSpec": { "name": "bigThing", "description": "Awesome method", '\
'  "inputSchema": { '\
'    "properties": { "dateParams": { "title": "Date parameters", "type": "string" } }, '\
'    "required": [], "type": "object" '\
'  } '\
'}'\
'}]'