use argh::FromArgs;
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::{process::Command, result, collections::HashMap, option::Option};
use tracing::{debug};
use typify_macro::import_types;


// TODO: Wrap this in mod mcp {...}?
#[cfg(feature = "mcp_20241105_schema")]
import_types!(schema="../../schemas/mcp_20241105_schema.json");
#[cfg(feature = "mcp_20250618_schema")]
import_types!(schema="../../schemas/mcp_20250618_schema.json");

/// MCP wrapper program
#[derive(FromArgs, Clone)]
pub struct Args {
    #[argh(option, short='t', description="array of tool specification command wrapper mappings in JSON format: [ {{ \"command\": \"scriptOrExecutable\", <\"command_parameters\": [ <\"mcp_param\": \"nameOfMcpMethodArgParameterToMapToCommandParam\">, <\"command_param\": \"staticCommandSwitchOrSwitchForMcpParameter\" ]>, \"mcp_tool_spec\": {{ mcpToolSpecJsonPerOfficialMcpSchema... }} }} , ... ]")]
    pub tool_specs: String, 
    #[cfg(feature = "debug_log")]
    #[argh(switch, short='d', description="debug output on stderr (will show up in console of MCP server/inspector)")]
    pub debug: bool,
    #[cfg(feature = "debug_log")]
    #[argh(switch, short='p', description="pretty print log (including console ASCII coloring)")]
    pub pretty: bool,    
    #[argh(switch, short='s', description="use sessions (via Mcp-Session-Id header)")]
    pub use_session: bool,
    #[argh(option, description="name of MCP server to declare to clients")]
    pub server_name: Option<String>,
    #[argh(option, description="version of MCP server to declare to clients")]
    pub server_version: Option<String>,
    #[cfg(feature = "network_client")]
    #[argh(option, default="String::from(\"0.0.0.0\")", description="host address to listen on")]
    pub host: String,
    #[cfg(feature = "network_client")]
    #[argh(option, default="3000", description="port to listen on")]
    pub port: u16  
}

pub fn get_args() -> Args {
    return argh::from_env();
}

pub fn conditionally_enable_debugging(args: &Args) {
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
}

/// Tool schema for passing to CLI
#[derive(Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    /// Shell script or executable program to execute
    command: String,
    /// List of command parameter mappings (either static switches or mapping of MCP method paremeters to command parameters)
    command_parameters: Option<Vec<CommandParameterMapping>>,
    /// MCP tool specification, compliant with MCP JSON-schema
    pub mcp_tool_spec: Tool
}
/// Command parameter (either static switch, and/or mapping from MCP method parameter to command switch or positional argument)
#[derive(Serialize, Deserialize, Clone)]
pub struct CommandParameterMapping {
    /// MCP method parameter name to map to
    mcp_param: Option<String>,
    /// Command line switch (either static or receiving MCP method argument value)
    command_param: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcServerResult {
    id: RequestId,
    jsonrpc: ::std::string::String,
    result: ServerResult
}

pub fn jsonrpc_error(request_id: RequestId, error_code: i64, message: String) -> JsonrpcError {
    return JsonrpcError {
        jsonrpc: "2.0".to_string(),
        id: request_id,
        error: JsonrpcErrorError {
            code: error_code,
            message: message,
            data: None
        }
    };
}

pub fn mcp_tools_list(id: RequestId, tools: &Vec<Tool>) -> JsonRpcServerResult { 
    return JsonRpcServerResult {
        jsonrpc: "2.0".to_string(),
        id: id,
        result: ServerResult::ListToolsResult(ListToolsResult {
            tools: tools.to_vec(),
            next_cursor: None,
            meta: json!({ }).as_object().unwrap().clone()
        })
    };
}

pub fn mcp_init(id: RequestId) -> JsonRpcServerResult {
    let empty_hash: HashMap<String, serde_json::Map<String, serde_json::Value>> = HashMap::new();
    let args = get_args();
    let server_name = &args.server_name.clone().or(Some(env!("CARGO_PKG_NAME").to_string())).unwrap();
    let server_version = &args.server_version.clone().or(Some(env!("CARGO_PKG_VERSION").to_string())).unwrap();
    return JsonRpcServerResult {
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
        };
}

pub fn mcp_handle_tool_call(id: RequestId, request: &CallToolRequest, tool_definition_map: &HashMap<String, ToolDefinition>) -> result::Result<JsonRpcServerResult, JsonrpcError> {
    let Some(tool) = tool_definition_map.get(&request.params.name) else { 
        return Err(jsonrpc_error(id, -32601, "Method name not found: ".to_owned() + &request.params.name));
    };
    let mut args: Vec<String> = Vec::new();
    // Collect args, both mapped method arguments and static command switches
    if tool.command_parameters.is_some() {
        for cp in tool.command_parameters.as_ref().unwrap().iter() {
            if cp.mcp_param.is_some() { 
                let arg_value: Option<&serde_json::Value> = request.params.arguments.get(cp.mcp_param.as_ref().unwrap());
                if arg_value.is_none() {  // They didn't pass this value -> just make call without it
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
    return Ok(JsonRpcServerResult {
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
        });
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