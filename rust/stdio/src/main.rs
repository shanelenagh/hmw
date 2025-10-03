use std::{process::Command, io::self, io::BufRead, result, error as std_error, collections::HashMap};
use argh::FromArgs;
use tracing::{debug};

use serde_json::json;
use serde::{Deserialize, Serialize};
use typify_macro::import_types;

import_types!(schema="../../schemas/mcp_20241105_schema.json");

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

/// Tool schema for passing to CLI
#[derive(Serialize, Deserialize)]
struct ToolDefinition {
    /// Shell script or executable program to execute
    command: String,
    /// List of command parameter mappings (either static switches or mapping of MCP method paremeters to command parameters)
    command_parameters: Option<Vec<CommandParameterMapping>>,
    /// MCP tool specification, compliant with MCP JSON-schema
    mcp_tool_spec: Tool
}
/// Command parameter (either static switch, and/or mapping from MCP method parameter to command switch or positional argument)
#[derive(Serialize, Deserialize)]
struct CommandParameterMapping {
    /// MCP method parameter name to map to
    mcp_param: Option<String>,
    /// Command line switch (either static or receiving MCP method argument value)
    command_param: Option<String>
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

#[derive(Serialize, Deserialize)]
struct JsonRpcServerResult {
    id: RequestId,
    jsonrpc: ::std::string::String,
    result: ServerResult
}

fn jsonrpc_error_str(request_id: RequestId, error_code: i64, message: String) -> result::Result<String, serde_json::Error> {
    return serde_json::to_string(&JsonrpcError {
        jsonrpc: "2.0".to_string(),
        id: request_id,
        error: JsonrpcErrorError {
            code: error_code,
            message: message,
            data: None
        }
    });
}

fn mcp_tools_list_string(id: RequestId, tools: &Vec<Tool>) -> result::Result<String, serde_json::Error> { 
    return serde_json::to_string(
        &JsonRpcServerResult {
            jsonrpc: "2.0".to_string(),
            id: id,
            result: ServerResult::ListToolsResult(ListToolsResult {
                tools: tools.to_vec(),
                next_cursor: None,
                meta: json!({ }).as_object().unwrap().clone()
            })
        }
    ); 
}

fn mcp_init_string(id: RequestId, server_name: &str, server_version: &str) -> result::Result<String,  serde_json::Error> {
    let empty_hash: HashMap<String, serde_json::Map<String, serde_json::Value>> = HashMap::new();
    return serde_json::to_string(
        &JsonRpcServerResult {
            jsonrpc: "2.0".to_string(),
            id: id,
            result: ServerResult::InitializeResult(
                InitializeResult {
                        instructions: None,
                        meta: json!({ }).as_object().unwrap().clone(),
                        protocol_version: "2024-11-05".to_string(),                    
                        capabilities: ServerCapabilities {
                            experimental: empty_hash.clone(),
                            prompts: Some(ServerCapabilitiesPrompts {
                                list_changed: Some(false)
                            }),
                            resources: Some(ServerCapabilitiesResources { 
                                subscribe: Some(false),
                                list_changed: Some(false)                        
                            }),
                            tools: Some(ServerCapabilitiesTools { 
                                list_changed: Some(false)
                            }),
                            logging: json!({ }).as_object().unwrap().clone()
                        },
                        server_info: Implementation {
                            name: server_name.to_string(),
                            version: server_version.to_string()
                        }
                    }
            )
        }
    );
}

fn mcp_handle_tool_call(id: RequestId, request: &CallToolRequest, tool_definition_map: &HashMap<String, &ToolDefinition>) -> result::Result<String, serde_json::Error> {
    let Some(tool) = tool_definition_map.get(&request.params.name) else { //TODO: Just make this a generic function and all these parsing things can call it
        return jsonrpc_error_str(id, -32601, "Method name not found: ".to_owned() + &request.params.name);
    };
    let mut args: Vec<String> = Vec::new();
    // Collect args, both mapped method arguments and static command switches
    if tool.command_parameters.is_some() {
        for cp in tool.command_parameters.as_ref().unwrap().iter() {
            if cp.mcp_param.is_some() { 
                let arg_value: Option<&serde_json::Value> = request.params.arguments.get(cp.mcp_param.as_ref().unwrap());
                if arg_value.is_none() {  // They didn't pass this value
                    continue;   
                }
                if cp.command_param.is_some() {
                    args.push(cp.command_param.as_ref().unwrap().to_owned());
                }
                args.push(arg_value.unwrap().to_owned().as_str().unwrap().to_owned());
            } else if cp.command_param.is_some() { 
                args.push(cp.command_param.as_ref().unwrap().to_owned());
            }            
        }
    }
    debug!("Executing command: {} with args: {:?}", tool.command, args);
    let exec_result = execute_process(&tool.command, args);
    let result_str = match exec_result {
        Ok(ref output) => output,
        Err(ref error) => error
    };
    debug!("Got result from execution: {}", result_str);
    return serde_json::to_string(
        &JsonRpcServerResult {
            jsonrpc: "2.0".to_string(),
            id: id,
            result: ServerResult::CallToolResult(CallToolResult {
                content: [ 
                    CallToolResultContentItem::TextContent(TextContent {
                        type_: "text".to_string(),
                        text: result_str.to_string(),
                        annotations: None
                    })
                ].to_vec(),
                is_error: Some(exec_result.is_err()),
                meta: json!({ }).as_object().unwrap().clone()
            })
        }        
    );
}

fn execute_process(command: &str, args: Vec<String>) -> result::Result<String,  String> {
    let output = Command::new(command)
        .args(args.as_slice())
        .output();
    match output {
        Ok(ok_output) => {
            if ok_output.status.success() {
                return Ok(String::from_utf8_lossy(&ok_output.stdout).to_string())
            } else {
                return Err(String::from_utf8_lossy(&ok_output.stderr).to_string())
            }            
        }
        Err(e) => {
            return Err("System level error: ".to_owned() + &e.to_string());
        }
    }
}