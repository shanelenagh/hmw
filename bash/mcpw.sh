#!/bin/bash
function errorUsage() {
    echo "USAGE: $0 [-t/--toolsSpecJason jsonToolsConfig] [-d/--debug] \
      [TODO: -r/--resourcesSpecJson jsonResourcesConfig] [TODO: -p/--promptsSpecJson jsonPromptsConfig]" >&2
    exit 1
}
function logDebug() {
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
    declare -A tool_command_param_map
    declare -A tool_required_args_map
    declare -A tool_mcp_input_property_map
    mapfile -t tools_array < <(echo "$TOOLS_SPEC_JSON" | jq -c '.[]' 2>/dev/null) 2>/dev/null
    for tool_json in "${tools_array[@]}"; do
        logDebug "Tool JSON: $tool_json"
        tool_name=$(echo "$tool_json" | jq -r '.mcpToolSpec.name' 2>/dev/null)
        tool_mcp_spec_map["$tool_name"]=$(echo "$tool_json" | jq -cr '.mcpToolSpec' 2>/dev/null)
        tool_commands_map["$tool_name"]=$(echo "$tool_json" | jq -r '.command' 2>/dev/null)
        tool_command_param_map["$tool_name"]=$(echo "$tool_json" | jq -cr '.commandParameters 
            | reduce .[] as $item (""; . + if . == "" then "" else "," end 
                + $item.mcpParameter+":"+$item.commandSwitch)'  2>/dev/null) 
        tool_required_args_map["$tool_name"]=$(echo "$tool_json" | jq -r '.mcpToolSpec.inputSchema.required 
            | reduce .[] as $item (""; . + if . == "" then "" else "," end + $item)' 2>/dev/null)
        tool_mcp_input_property_map["$tool_name"]=$(echo "$tool_json" | jq -cr '.mcpToolSpec.inputSchema.properties 
            | to_entries | map("[\(.key)]=\"\(.value.type)\"") | join(" ")' 2>/dev/null) # for easy associative array deref later
    done
fi
# TODO: Validate that all MCP defined params have command param mappings
if [[ -n "$DEBUG" ]]; then
    for k in "${!tool_mcp_spec_map[@]}"; do
        logDebug "Loaded tool with command [${tool_commands_map[$k]}]"
        logDebug "    and mcp spec [${tool_mcp_spec_map[$k]}]"
        logDebug "    and param mapping [${tool_command_param_map[$k]}]"
        logDebug "    and required args [${tool_required_args_map[$k]}]"
        logDebug "    and mcp input properties [${tool_mcp_input_property_map[$k]}]"
    done
fi
#exit 0

# Main loop to read JSON-RPC MCP requests from stdin
while read -r line; do
    # Parse JSON input using jq
    method=$(echo "$line" | jq -r '.method' 2>/dev/null)
    id=$(echo "$line" | jq -r '.id' 2>/dev/null)
    if [[ "$method" == "initialize" ]]; then
        echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"protocolVersion":"2024-11-05","capabilities":'\
            '{"experimental":{},"prompts":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false},'\
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
        logDebug "Tool name of $tool_name and command ${tool_commands_map[$tool_name]}"
        commandString=${tool_commands_map[$tool_name]}
        #paramString="-u --debug" # TODO: Piece together with mapping and request params
        declare -A passed_args_map="( $(echo "$line" \
            | jq -cr '.params.arguments | to_entries |  map("[\(.key)]=\"\(.value)\"") | join(" ")' ) )"
        if [[ -n "$DEBUG" ]]; then
            for arg in "${!passed_args_map[@]}"; do
                logDebug "Passed param key: $arg, Value: ${passed_args_map[$arg]}"
            done
        fi
        for arg in "${!passed_args_map[@]}"; do
            logDebug "Passed param key: $arg, Value: ${passed_args_map[$arg]}"
        done
        # Check for missing required args, per MCP input properties json schema
        declare -a missing_args_array
        IFS=',' 
        for reqArg in ${tool_required_args_map[$tool_name]}; do
            logDebug "Required arg: $reqArg"
            if [[ -z "${passed_args_map[$reqArg]}" ]]; then
                missing_args_array+=("\"$reqArg\"")
            fi
        done
        if [[ ${#missing_args_array[@]} -gt 0 ]]; then
            echo '{"jsonrpc":"2.0","id":'"$id"',"error":{"code":-32602,"message":"Invalid params",'\
                '"data":{"missingParameters":[' ${missing_args_array} ']}}}'
            unset IFS
            continue
        fi
        unset IFS
        # Now that we know we have required params, we get to work on creating param string
        declare -A paramTypes=( $(echo "${tool_mcp_input_property_map[$tool_name]}") )
        paramString=""
        logDebug "Command param mapping string: ${tool_command_param_map[$tool_name]}"
        IFS=','; tool_command_params=(${tool_command_param_map[$tool_name]})
        unset IFS
        for paramMapping in "${tool_command_params[@]}"; do
            logDebug "Command param mapping key: $paramMapping"
            IFS=':' ; paramMappingParts=($paramMapping) # Split on colon
            unset IFS
            logDebug "part 0=${paramMappingParts[0]}, part 1=${paramMappingParts[1]}"
            paramString+=" ${paramMappingParts[1]}"
            if [[ -n "${paramMappingParts[0]}" ]]; then
                # TODO: Use paramTypes to decide formatting/quoting
                paramString+="${passed_args_map[${paramMappingParts[0]}]}"
            #else: it's a static switch, so nothing more to do
            fi
        done
        #paramString="-u --debug" # TODO: Piece together with mapping and request params
        logDebug "Executing command [$commandString] with params: $paramString"
        result=$($commandString $paramString | jq -R @json)
        echo '{"jsonrpc":"2.0","id":'"$id"',"result":{"content":[{"type":"text","text":'$result'}],"isError":false}}'
    else
        echo '{"jsonrpc":"2.0","id":'"$id"',"error":{"code":-32601,"message":"Method not found"}}'
    fi
done || break