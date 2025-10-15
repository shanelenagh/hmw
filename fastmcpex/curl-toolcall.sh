#!/bin/sh
curl http://localhost:8000/mcp "$@" \
    -H "Accept: text/event-stream, application/json" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/call", 
        "params": { "name": "greet", 
            "arguments": { "name": "Joedaddy" } } }'