#!/bin/sh
curl -v http://localhost:8000/mcp "$@" \
    -H "Accept: text/event-stream, application/json" \
    -H "Content-Type: application/json" \
    -d '{ "jsonrpc": "2.0", "method": "initialize", "id": 1, "params": { "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "myClient", "version": "0.1" }}}'