#!/bin/bash
cargo build --release && while IFS= read -r line; do
    echo "$line" | ./target/release/mcpws.exe -t '[ { "command": "date", "command_parameters": [{ "mcp_parameter": "dateParams" }], "mcp_tool_spec": { "name": "bigThing", "description": "Awesome method",     "inputSchema": { "properties": { "dateParams": { "title": "Date parameters", "type": "string" } }, "required": [], "type": "object" } } } ]'
done
# Init: { "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2024-11-05", "capabilities": { }, "clientInfo": { "name": "shaner", "version": "1.0.0" } } }
# tool call: { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "bigThing", "arguments": { "dateParams": "-u" } } }
# tool call: { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "bigThing", "arguments": { "dateParams": "+%F" } } }

# ./target/release/mcpws.exe -t '[ { "command": "C:\\Program Files\\Git\\bin\\bash.exe", "command_parameters": [ { "command_switch": "./hi.sh" }, { "command_switch": "big boy", "mcp_parameter": "name" } ], "mcp_tool_spec": { "name": "hi", "description": "Echo hello", "inputSchema": { "properties": { }, "required": [], "type": "object" } } } ]'
# { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "hi", "arguments": { "name": "Shane Lenagh" } } }

# ./target/release/mcpws.exe -t '[ { "command": "C:\\Program Files\\Git\\bin\\bash.exe", "command_parameters": [ { "command_switch": "/c/Users/ctr-slenagh/projects/voice-rag/image-rag.sh" }, { "command_switch": "-i", "mcp_parameter": "imageFilePath" }, { "command_switch": "-p", "mcp_parameter": "prompt" } ], "mcp_tool_spec": { "name": "Image RAG", "description": "Image question answering service", "inputSchema": { "properties": { "imageFilePath": { "type": "string", "title": "Image file path" }, "prompt": { "type": "string", "title": "Question to ask about the image" } }, "required": [ "imageFilePath", "prompt" ], "type": "object" } } } ]'
# { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "Image RAG", "arguments": { "imageFilePath": "/c/Users/ctr-slenagh/projects/voice-rag/UBClaim.png", "prompt": "Who is the patient?" } } }