#!/bin/bash
cargo build && while IFS= read -r line; do
    echo "$line" | RUST_LOG=debug ./target/debug/mcpws -t '[ { "command": "date", "command_parameters": [{ "mcp_param": "dateParams" }], "mcp_tool_spec": { "name": "bigThing", "description": "Awesome method",     "inputSchema": { "properties": { "dateParams": { "title": "Date parameters", "type": "string" } }, "required": [], "type": "object" } } } ]'
done
# Init: { "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2024-11-05", "capabilities": { }, "clientInfo": { "name": "shaner", "version": "1.0.0" } } }
# tool call: { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "bigThing", "arguments": { "dateParams": "-u" } } }
# tool call: { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "bigThing", "arguments": { "dateParams": "+%F" } } }

# ./target/release/mcpws.exe -t '[ { "command": "C:\\Program Files\\Git\\bin\\bash.exe", "command_parameters": [ { "command_param": "./hi.sh" }, { "command_param": "big boy", "mcp_param": "name" } ], "mcp_tool_spec": { "name": "hi", "description": "Echo hello", "inputSchema": { "properties": { }, "required": [], "type": "object" } } } ]'
# { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "hi", "arguments": { "name": "Shane Lenagh" } } }

# ./target/release/mcpws.exe -t '[ { "command": "C:\\Program Files\\Git\\bin\\bash.exe", "command_parameters": [ { "command_param": "/c/Users/ctr-slenagh/projects/voice-rag/image-rag.sh" }, { "command_param": "-i", "mcp_param": "imageFilePath" }, { "command_param": "-p", "mcp_param": "prompt" } ], "mcp_tool_spec": { "name": "Image RAG", "description": "Image question answering service", "inputSchema": { "properties": { "imageFilePath": { "type": "string", "title": "Image file path" }, "prompt": { "type": "string", "title": "Question to ask about the image" } }, "required": [ "imageFilePath", "prompt" ], "type": "object" } } } ]'
# { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "Image RAG", "arguments": { "imageFilePath": "/c/Users/ctr-slenagh/projects/voice-rag/UBClaim.png", "prompt": "Who is the patient?" } } }