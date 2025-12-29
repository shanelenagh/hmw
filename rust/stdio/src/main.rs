use serde_json::{from_str, to_string};
use std::{io::self, io::BufRead, result, error as std_error, collections::HashMap};
use tracing::{debug};
use mcpw::*;

fn main() -> result::Result<(), Box<dyn std_error::Error>> {
    let args = get_args();
    conditionally_enable_debugging(&args);
    debug!("Tool specs passed in: {}", args.tool_specs);
    let Ok(tool_definitions) = from_str::<Vec<ToolDefinition>>(&args.tool_specs) else {
        return Err(("Can't parse tool list (confirm schema with help CLI option): ".to_owned() + &args.tool_specs).into());
    };
    let tool_spec_map: HashMap<String, ToolDefinition> = tool_definitions.iter()
        .map(|tool| (tool.mcp_tool_spec.name.clone(), tool.clone())).collect();
    let mcp_tools = tool_definitions.iter().map(|tool| tool.mcp_tool_spec.clone()).collect::<Vec<Tool>>();

    for line_result in io::stdin().lock().lines() {
        let line = line_result?;
        let Ok(jsonrpc_request) = from_str::<JsonrpcRequest>(&line) else {
            println!("{}", to_string(&jsonrpc_error(
                RequestId::from(-1), -32700, "Parsing of request failed (check conformance with MCP Schema): ".to_string() + &line))?);     
            continue;       
        };
        debug!("Received line: {} with method {}", line, jsonrpc_request.method);
        match jsonrpc_request.method.as_str() {
            "initialize" => {
                println!("{}", to_string(&mcp_init(jsonrpc_request.id))?);
            },
            "tools/call" => {
                let Ok(tool_call_request) = from_str::<CallToolRequest>(&line) else {
                    println!("{}", to_string(&jsonrpc_error(
                        RequestId::from(-1), -32700, "Parsing of tool call request failed: ".to_string()+&line))?);
                    continue;
                };
                println!("{}", to_string(&mcp_handle_tool_call(jsonrpc_request.id, &tool_call_request, &tool_spec_map))?);
            },
            "tools/list" => {
                println!("{}", to_string(&mcp_tools_list(jsonrpc_request.id, &mcp_tools))?);
            },
            _ => {
                println!("{}", to_string(&jsonrpc_error(
                    jsonrpc_request.id, -32601, "MCP method not found: ".to_string() + jsonrpc_request.method.as_str()))?);
            }
        }     
    }
    return Ok(())
}