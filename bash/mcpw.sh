#!/bin/bash
function errorUsage() {
    echo "USAGE: $0 [-t/--toolsSpecJason jsonToolsConfig] [-d/--debug] \
      [TODO: -r/--resourcesSpecJson jsonResourcesConfig] [TODO: -p/--promptsSpecJson jsonPromptsConfig]" >&2
    exit 1
}
function debugLog() {
    if [[ -n "$DEBUG" ]]; then
        echo "$@" >&2
    fi
}
while [[ $# -gt 0 ]]; do
  case $1 in
    -t|--toolsSpecJason)
      TOOLS_SPEC_JSON="$2"
      shift; shift;;
    -s|--serverName)
      SERVER_NAME="$2"
      shift; shift;;
    -r|--resourcesSpecJson)
      echo "Not implemented yet" >&2 && errorUsage;;      
    -p|--promptsSpecJson)
      echo "Not implemented yet" >&2 && errorUsage;;
    -d|--debug)
      DEBUG="true"
      shift;;   
    *)
      echo "Invalid option: $1" >&2 && errorUsage;;
  esac
done

# Load tool properties from JSON spec into associative arrays
# SCHEMA: [ { command: str, <commandParameters>=[{<mcpParameter: str>, <commandSwitch: str>}, ...]>, mcpToolSpec: {mcpToolSchema: https://modelcontextprotocol.io/specification/2025-06-18/server/tools#tool} } ]
if [[ -n "$TOOLS_SPEC_JSON" ]]; then
    declare -A tool_commands_map
    declare -A tool_mcp_spec_map
    declare -A tool_command_param_mapping_map
    mapfile -t tools_array < <(echo "$TOOLS_SPEC_JSON" | jq -c '.[]' 2>/dev/null) 2>/dev/null
    for tool_json in "${tools_array[@]}"; do
        debugLog "Tool JSON: $tool_json"
        tool_name=$(echo "$tool_json" | jq -r '.mcpToolSpec.name' 2>/dev/null)
        tool_mcp_spec_map["$tool_name"]=$(echo "$tool_json" | jq -cr '.mcpToolSpec' 2>/dev/null)
        tool_commands_map["$tool_name"]=$(echo "$tool_json" | jq -r '.command' 2>/dev/null)
        tool_command_param_mapping_map["$tool_name"]=$(echo "$tool_json" | jq -cr '.commandParameters 
            | reduce .[] as $item (""; . + if . == "" then "" else "," end 
                + $item.mcpParameter+":"+$item.commandSwitch)'  2>/dev/null) 
    done
fi
for k in "${!tool_mcp_spec_map[@]}"; do
    debugLog "Loaded tool with command [${tool_commands_map[$k]}]"
    debugLog "    and mcp spec [${tool_mcp_spec_map[$k]}]"
    debugLog "    and param mapping [${tool_command_param_mapping_map[$k]}]"
done
#exit 0

# Main loop to read JSON-RPC MCP requests from stdin
while read -r line; do
    # Parse JSON input using jq
    method=$(echo "$line" | jq -r '.method' 2>/dev/null)
    id=$(echo "$line" | jq -r '.id' 2>/dev/null)
    if [[ "$method" == "initialize" ]]; then
        echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"protocolVersion":"2024-11-05","capabilities":{"experimental":{},'\
            '"prompts":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false},'\
            '"tools":{"listChanged":false}},"serverInfo":{"name":"'"${SERVER_NAME:-"hmcpw"}"'","version":"0.0.1"}}}' 
    elif [[ "$method" == "notifications/initialized" ]]; then
        : #do nothing
    elif [[ "$method" == "tools/list" ]]; then
        toolsList='{"jsonrpc":"2.0","id":'"$id"',"result":{"tools":['
        i=0
        for tool in "${!tool_mcp_spec_map[@]}"; do
            if [[ $i -gt 0 ]]; then
                toolsList+=','
            fi
            toolsList+="${tool_mcp_spec_map[$tool]}"
            i=i+1
        done
        toolsList+=']}}'
        echo $toolsList
    elif [[ "$method" == "resources/list" ]]; then
        echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"resources":[]}}'
    elif [[ "$method" == "prompts/list" ]]; then
        echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"prompts":[]}}'
    elif [[ "$method" == "tools/call" ]]; then
        tool_name=$(echo "$line" | jq -r '.params.name' 2>/dev/null)
        debugLog "Tool name of $tool_name and command ${tool_commands_map[$tool_name]}"
        commandString=${tool_commands_map[$tool_name]}
        #paramString="-u --debug" # TODO: Piece together with mapping and request params
        paramString=""
        IFS=',' read -ra paramSpecArr <<< "${tool_command_param_mapping_map[$tool_name]}"
        for paramSpec in "${paramSpecArr[@]}"; do
            debugLog "Got spec $paramSpec"
        done
        for arg in $(echo "$line" | jq -cr '.params.arguments | to_entries[]'); do
            debugLog "Got arg $arg"
        done
        paramString="-u --debug" # TODO: Piece together with mapping and request params
        result=$($commandString $paramString | jq -R @json)
        echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"content":[{"type":"text","text":'$result'}],"isError":false}}'
    else
        echo '{"jsonrpc":"2.0","id":'"$id"',"error":{"code":-32601,"message":"Method not found"}}'
    fi
done || break