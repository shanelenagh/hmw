use std::process::Command;
use std::io::{self, BufRead};
use std::result;
use argh::FromArgs;
use std::collections::HashMap;

use serde_json::json;
use serde::{Deserialize, Serialize};
use typify_macro::import_types;

import_types!(schema="../schemas/mcp_20241105_schema.json");

/// stdio-transport MCP server that wraps a local command
#[derive(FromArgs)]
struct Args {
    #[argh(option, short='t', description="array of tool specification command wrapper mappings in JSON format: [ {{ \"command\": \"scriptOrExecutable\", <\"command_parameters\": [ <\"mcp_parameter\": \"nameOfMcpMethodArgParameterToMapToCommandParam\">, <\"command_switch\": \"staticCommandSwitchOrSwitchForMcpParameter\" ]>, \"mcp_tool_spec\": {{ mcpToolSpecJsonPerMcpSchema... }} }} , ... ]")]
    tool_specs: String 
}

/// Tool schema for passing to CLI
#[derive(Serialize, Deserialize)]
struct ToolDefinition {
    /// Shell script or executable program to execute
    command: String,
    /// List of command parameter mappings (either static switches or mapping of MCP method paremeters to command parameters)
    command_parameters: Option<Vec<CommandParameter>>,
    /// MCP tool specification, compliant with MCP JSON-schema
    mcp_tool_spec: Tool
}
/// Command parameter (either static switch, and/or mapping from MCP method parameter to command switch or positional argument)
#[derive(Serialize, Deserialize)]
struct CommandParameter {
    /// MCP method parameter name to map to
    mcp_parameter: Option<String>,
    /// Command line switch (either static or receiving MCP method argument value)
    command_switch: Option<String>
}

fn main() -> io::Result<()> {
    let args: Args = argh::from_env();
    eprintln!("Tool specs passed in: {}", args.tool_specs);
    let deserialized_tools: Vec<ToolDefinition> = serde_json::from_str(&args.tool_specs)?;
    let tool_spec_map: HashMap<String, &ToolDefinition> = deserialized_tools.iter()
        .map(|tool| (tool.mcp_tool_spec.name.clone(), tool)).collect();

    let stdin_handle = io::stdin().lock();
    for line_result in stdin_handle.lines() {
        let line = line_result?;
        let jsonrpc_request: JsonrpcRequest = serde_json::from_str(&line)?;
        // TODO: Use logging framework debug statements for these
        eprintln!("Received line: {} with method {}", line, jsonrpc_request.method);
        match jsonrpc_request.method.as_str() {
            "initialize" => {
                let init_string = mcp_init_string(jsonrpc_request.id, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))?;
                println!("{}", init_string);
                eprintln!("{}", init_string);
            },
            "tools/call" => {
                let tool_call_request: CallToolRequest = serde_json::from_str(&line)?;
                let tool_call_response = mcp_handle_tool_call(jsonrpc_request.id, &tool_call_request, &tool_spec_map)?;
                println!("{}", tool_call_response);
            },
            "tools/list" => {
                let tools_list_response = mcp_tools_list_string(jsonrpc_request.id, &deserialized_tools)?;
                println!("{}", tools_list_response);
            },
            _ => {
                let error_response = JsonrpcError {
                    jsonrpc: "2.0".to_string(),
                    id: jsonrpc_request.id,
                    error: JsonrpcErrorError {
                        code: -32601,
                        message: "Method not found".to_string(),
                        data: None
                    }
                };
                let error_text = serde_json::to_string(&error_response)?;
                println!("{}", error_text);
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

fn mcp_tools_list_string(id: RequestId, deserialized_tools: &Vec<ToolDefinition>) -> result::Result<String, serde_json::Error> { 
    return serde_json::to_string(
        &JsonRpcServerResult {
            jsonrpc: "2.0".to_string(),
            id: id,
            result: ServerResult::ListToolsResult(ListToolsResult {
                tools: deserialized_tools.iter().map(|tool| tool.mcp_tool_spec.clone()).collect::<Vec<Tool>>(),
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
    let tool: &ToolDefinition = tool_definition_map.get(&request.params.name).expect("Tool not found in map");
    let mut args: Vec<String> = Vec::new();
    // Coolect args, both mapped method arguments and static command switches
    if tool.command_parameters.is_some() {
        for cp in tool.command_parameters.as_ref().unwrap().iter() {
            if cp.mcp_parameter.is_some() { 
                let arg_value: Option<&serde_json::Value> = request.params.arguments.get(cp.mcp_parameter.as_ref().unwrap());
                if arg_value.is_none() {
                    continue;   // They didn't pass this value
                }
                if cp.command_switch.is_some() {
                    args.push(cp.command_switch.as_ref().unwrap().to_owned());
                }
                args.push(arg_value.unwrap().to_owned().as_str().unwrap().to_owned());
            } else if cp.command_switch.is_some() { 
                args.push(cp.command_switch.as_ref().unwrap().to_owned());
            }            
        }
    }
    eprintln!("Executing command: {} with args: {:?}", tool.command, args);
    let exec_result = execute_process(&tool.command, args);
    let result_str = match exec_result {
        Ok(ref output) => output,
        Err(ref error) => error
    };
    eprintln!("Got result from execution: {}", result_str);
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