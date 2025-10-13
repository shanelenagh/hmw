use std::{io::self, io::BufRead, result, error as std_error, collections::HashMap};
use argh::FromArgs;
use tracing::{debug};
use mcpw_common::*;


/// stdio-transport MCP server that wraps a local command
#[derive(FromArgs)]
struct Args {
    #[argh(option, short='t', description="array of tool specification command wrapper mappings in JSON format: [ {{ \"command\": \"scriptOrExecutable\", <\"command_parameters\": [ <\"mcp_param\": \"nameOfMcpMethodArgParameterToMapToCommandParam\">, <\"command_param\": \"staticCommandSwitchOrSwitchForMcpParameter\" ]>, \"mcp_tool_spec\": {{ mcpToolSpecJsonPerOfficialMcpSchema... }} }} , ... ]")]
    tool_specs: String, 
    #[cfg(feature = "debug_log")]
    #[argh(switch, short='d', description="debug output on stderr (will show up in console of MCP server/inspector)")]
    debug: bool,
    #[cfg(feature = "debug_log")]
    #[argh(switch, short='p', description="pretty print log (including console ASCII coloring)")]
    pretty: bool
}

fn main() -> result::Result<(), Box<dyn std_error::Error>> {
    let args: Args = argh::from_env();
    #[cfg(feature = "debug_log")]
    if args.debug {
        use tracing_subscriber::{fmt, prelude::*};
        if args.pretty {
            tracing_subscriber::registry().with(
                fmt::layer()
                    .pretty()                   
                    .with_writer(std::io::stderr)   // Specify stderr as the output target
            ).init();
        } else {
            tracing_subscriber::registry().with(
                fmt::layer()
                    .with_ansi(false)
                    .with_writer(std::io::stderr)  // Specify stderr as the output target
            ).init();
        }
    }    
    debug!("Tool specs passed in: {}", args.tool_specs);
    let Ok(tool_definitions) = serde_json::from_str::<Vec<ToolDefinition>>(&args.tool_specs) else {
        return Err(("Can't parse tool list (confirm schema with help CLI option): ".to_owned() + &args.tool_specs).into());
    };
    let tool_spec_map: HashMap<String, &ToolDefinition> = tool_definitions.iter()
        .map(|tool| (tool.mcp_tool_spec.name.clone(), tool)).collect();
    let mcp_tools = tool_definitions.iter().map(|tool| tool.mcp_tool_spec.clone()).collect::<Vec<Tool>>();

    let stdin_handle = io::stdin().lock();
    for line_result in stdin_handle.lines() {
        let line = line_result?;
        let Ok(jsonrpc_request) = serde_json::from_str::<JsonrpcRequest>(&line) else {
            println!("{}", jsonrpc_error_str(RequestId::from(-1), -32700, "Parsing of request failed (check conformance with MCP Schema): ".to_string() + &line)?);     
            continue;       
        };
        debug!("Received line: {} with method {}", line, jsonrpc_request.method);
        match jsonrpc_request.method.as_str() {
            "initialize" => {
                println!("{}", mcp_init_string(jsonrpc_request.id, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))?);
            },
            "tools/call" => {
                let Ok(tool_call_request) = serde_json::from_str::<CallToolRequest>(&line) else {
                    println!("{}", jsonrpc_error_str(RequestId::from(-1), -32700, "Parsing of tool call request failed: ".to_string()+&line)?);
                    continue;
                };
                println!("{}", mcp_handle_tool_call(jsonrpc_request.id, &tool_call_request, &tool_spec_map)?);
            },
            "tools/list" => {
                println!("{}", mcp_tools_list_string(jsonrpc_request.id, &mcp_tools)?);
            },
            _ => {
                println!("{}", jsonrpc_error_str(jsonrpc_request.id, -32601, "MCP method not found: ".to_string() + jsonrpc_request.method.as_str())?);
            }
        }     
    }
    return Ok(())
}