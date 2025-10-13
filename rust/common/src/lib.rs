use serde_json::json;
use serde::{Deserialize, Serialize};
use typify_macro::import_types;
use std::{process::Command, result, collections::HashMap};
use tracing::{debug};

// TODO: Wrap this in mod mcp {...}?
// TODO: Put feature conditional here, for version of schema to use
import_types!(schema="../../schemas/mcp_20241105_schema.json");

/// Tool schema for passing to CLI
#[derive(Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Shell script or executable program to execute
    command: String,
    /// List of command parameter mappings (either static switches or mapping of MCP method paremeters to command parameters)
    command_parameters: Option<Vec<CommandParameterMapping>>,
    /// MCP tool specification, compliant with MCP JSON-schema
    pub mcp_tool_spec: Tool
}
/// Command parameter (either static switch, and/or mapping from MCP method parameter to command switch or positional argument)
#[derive(Serialize, Deserialize)]
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

pub fn jsonrpc_error_str(request_id: RequestId, error_code: i64, message: String) -> result::Result<String, serde_json::Error> {
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

pub fn mcp_tools_list_string(id: RequestId, tools: &Vec<Tool>) -> result::Result<String, serde_json::Error> { 
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

pub fn mcp_init_string(id: RequestId, server_name: &str, server_version: &str) -> result::Result<String,  serde_json::Error> {
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

pub fn mcp_handle_tool_call(id: RequestId, request: &CallToolRequest, tool_definition_map: &HashMap<String, &ToolDefinition>) -> result::Result<String, serde_json::Error> {
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

pub fn execute_process(command: &str, args: Vec<String>) -> result::Result<String,  String> {
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


// TODO: Embedded tests baby, like below

// pub fn add(left: u64, right: u64) -> u64 {
//     left + right
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
