use std::process::Command;
use std::io::{self, BufRead};
use std::result;
use argh::FromArgs;

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
    #[argh(option, short='t', description="array of tool specification command wrapper mappings in JSON format")]
    tool_specs: String 
}

fn main() -> io::Result<()> {
    let args: Args = argh::from_env();
    println!("Tool specs passed in: {}", args.tool_specs);
    let deserialized_tools: Vec<ToolDefinition> = serde_json::from_str(&args.tool_specs)?;

    let stdin_handle = io::stdin().lock();
    for line_result in stdin_handle.lines() {
        let line = line_result?;
        println!("Received line: {}", line);
        let result = execute_process(&(deserialized_tools[0]).command, &[]); 
        println!("Process exited with: Output:\n{:?}", result);        
    }

    Ok(())
}

fn execute_process(command: &str, args: &[&str]) -> result::Result<String,  String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        return Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}