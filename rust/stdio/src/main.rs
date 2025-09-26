use std::process::Command;
use std::io::{self, BufRead};
use std::result;
use argh::FromArgs;
use std::collections::HashMap;

use serde_json::json;
use serde::{Deserialize, Serialize};
use typify_macro::import_types;

import_types!(schema="../schemas/mcp_20241105_schema.json");

/// Tool schema for passing to CLI
#[derive(Serialize, Deserialize, Debug)]
struct ToolDefinition {
    /// Shell script or executable program to execute
    command: String,
    /// List of command parameter mappings (either static switches or mapping of MCP method paremeters to command parameters)
    command_parameters: Option<Vec<CommandParameter>>,
    /// MCP tool specification, compliant with MCP JSON-schema
    mcp_tool_spec: Tool
}
/// Command parameter (either static switch, and/or mapping from MCP method parameter to command switch or positional argument)
#[derive(Serialize, Deserialize, Debug)]
struct CommandParameter {
    /// MCP method parameter name to map to
    mcp_parameter: Option<String>,
    /// Command line switch (either static or receiving MCP method argument value)
    command_switch: Option<String>
}

#[derive(FromArgs, Debug)]
/// stdio-transport MCP server that wraps a local command
struct Args {
    #[argh(option, short='t', description="array of tool specification command wrapper mappings in JSON format: [ {{ \"command\": \"scriptOrExecutable\", <\"command_parameters\": [ <\"mcp_parameter\": \"nameOfMcpMethodArgParameterToMapToCommandParam\">, <\"command_switch\": \"staticCommandSwitchOrSwitchForMcpParameter\" ]>, \"mcp_tool_spec\": {{ mcpToolSpecJsonPerMcpSchema... }} }} , ... ]")]
    tool_specs: String 
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
                let init_string = mcp_init_string(jsonrpc_request.id, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), &deserialized_tools)?;
                println!("{}", init_string);
                continue;
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
                continue;
            }
        }     
    }
    return Ok(())
}

fn mcp_tools_list_string(id: RequestId, deserialized_tools: &Vec<ToolDefinition>) -> result::Result<String, serde_json::Error> {
    let tools_list_json = JsonrpcResponse {
        jsonrpc: "2.0".to_string(),
        id: id,
        result: Result {
            extra: json!({
                "tools": deserialized_tools.iter().map(|tool| &tool.mcp_tool_spec).collect::<Vec<&Tool>>()
            }).as_object().unwrap().clone(),
            meta: json!({ }).as_object().unwrap().clone()
        }
    };   
    return serde_json::to_string(&tools_list_json); 
}

fn mcp_init_string(id: RequestId, server_name: &str, server_version: &str, deserialized_tools: &Vec<ToolDefinition>) -> result::Result<String,  serde_json::Error> {
    let init_json = JsonrpcResponse {
        jsonrpc: "2.0".to_string(),
        id: id,
        result: Result {
            extra: json!({
                "capabilities": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": server_name,
                        "version": server_version
                    },
                    "tools": deserialized_tools.iter().map(|tool| &tool.mcp_tool_spec).collect::<Vec<&Tool>>()
                }
            }).as_object().unwrap().clone(),
            meta: json!({
                "serverInfo": {
                    "name": server_name,
                    "version": server_version
                }
            }).as_object().unwrap().clone()
        }
    };
    return serde_json::to_string(&init_json);
}

fn mcp_handle_tool_call(id: RequestId, request: &CallToolRequest, tool_definition_map: &HashMap<String, &ToolDefinition>) -> result::Result<String, serde_json::Error> {
    let tool: &ToolDefinition = tool_definition_map.get(&request.params.name).expect("Tool not found in map");
    let mut args: Vec<String> = Vec::new();
    //TODO: all these daisy chained '.to_owned().as_str().unwrap().to_owned()''s are a hot mess--gotta be a simpler way
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
                args.push(arg_value.unwrap().to_owned().as_str().unwrap().to_owned());  // TODO: Holy crap this daisy chain is obtuse!!!
            } else if cp.command_switch.is_some() { 
                args.push(cp.command_switch.as_ref().unwrap().to_owned());
            }            
        }
    }
    eprintln!("Executing command: {} with args: {:?}", tool.command, args);
    let result = execute_process(&tool.command, args /* TODO: Put in params */).expect("Failed to execute process"); 
    eprintln!("Process exited with output: {:?}", result);                   
    return Ok(("{ \"result\": \"".to_owned() + &result + "\", \"id\": " + &id.to_string() + " }").to_string())
}

fn execute_process(command: &str, args: Vec<String>) -> result::Result<String,  String> {
    let output = Command::new(command)
        .args(args.as_slice())
        .output()
        .expect("Failed to execute command");
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        return Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}