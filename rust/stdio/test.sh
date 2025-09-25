cargo build --release && while IFS= read -r line; do
    echo "$line" | ./target/release/mcpws.exe -t '[ { "command": "date", "commandParameters": [{ "mcpParameter": "dateParams" }, { "commandSwitch": "-R" }], "mcp_tool_spec": { "name": "bigThing", "description": "Awesome method",     "inputSchema": { "properties": { "dateParams": { "title": "Date parameters", "type": "string" } }, "required": [], "type": "object" } } } ]'
done
# Init: { "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2024-11-05", "capabilities": { }, "clientInfo": { "name": "shaner", "version": "1.0.0" } } }
# tool call: { "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "dateParams": "now" } }