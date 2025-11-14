#!/bin/bash
curl http://localhost:3000/mcp -H "Content-Type: application/json" -d '
    { "jsonrpc": "2.0", "id": 2, "method": "tools/call", 
        "params": { "name": "bigThing", "arguments": { "dateParams": "-u" } } }'