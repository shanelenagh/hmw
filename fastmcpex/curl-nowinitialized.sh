#!/bin/sh
curl "$@" -X POST http://localhost:8000/mcp \
    -H "Accept: text/event-stream, application/json" \
    -H "Content-Type: application/json" \
    -d '{ "jsonrpc": "2.0", "method": "notifications/initialized" }'