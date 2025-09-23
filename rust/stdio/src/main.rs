// use std::process::Command;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;


use typify_macro::import_types;
//use serde_json;
import_types!(
    schema="schemas/mcp_20241105_schema.json",
    derives=[schemars::JsonSchema],
    struct_builder = true
);

// Tool schema for passing to CLI
#[derive(Serialize, Deserialize, Debug)]
struct ToolDefinition<'a> {
    command: &'a str,
    command_parameters: Option<Vec<CommandParameter<'a>>>,
    mcp_tool_spec: Tool
}
#[derive(Serialize, Deserialize, Debug)]
struct CommandParameter<'a> {
    mcp_parameter: Option<&'a str>,
    command_switch: Option<&'a str>
}

fn main() -> serde_json::Result<()> {
    // //println!("Hello, world!");

    // // Call an external program (e.g., 'ls' on Unix-like systems, 'dir' on Windows)
    // let output = Command::new("ls") // Or "dir" on Windows
    //     .arg("-l") // Add an argument
    //     .output()
    //     .expect("Failed to execute command");

    // if output.status.success() {
    //     println!("Command executed successfully!");
    //     println!("Output:\n{}", String::from_utf8_lossy(&output.stdout));
    // } else {
    //     eprintln!("Command failed with error:");
    //     eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    // }

    // // Spawning a process and waiting for it later
    // let mut child = Command::new("sleep") // Or "timeout /t 5" on Windows
    //     .arg("5")
    //     .spawn()
    //     .expect("Failed to spawn child process");

    // println!("Child process spawned, waiting for it to finish...");
    // let status = child.wait().expect("Failed to wait on child process");
    // println!("Child process exited with status: {:?}", status);
    
    
    /*
     * Now on to JSON demonstration
     */
    // 1. Serialization: Convert a Rust struct to a JSON string.
    // let person = Person {
    //     name: "John Doe".to_string(),
    //     age: 43,
    //     phones: vec!["+44 1234567".to_string(), "+44 2345678".to_string()],
    // };

    let tools = [
        ToolDefinition {
            command: "echo",
            command_parameters: Some(vec![
                CommandParameter {
                    mcp_parameter: Some("param1"),
                    command_switch: Some("-n")
                }
            ]),
            mcp_tool_spec: Tool {
                name: "echo".to_string(),
                description: Some("A tool that echoes input".to_string()),
                input_schema: ToolInputSchema {
                    type_: "object".to_string(),
                    required: Vec::<String>::new(),
                    properties: HashMap::new(),
                }
            }
        }];

    let json_string = serde_json::to_string_pretty(&tools)?; // `to_string_pretty` for formatted output
    println!("Serialized JSON:\n{}", json_string);

    // // 2. Deserialization: Convert a JSON string to a Rust struct.

    // let json_data = r#"
    //     {
    //         "name": "Jane Smith",
    //         "age": 30,
    //         "phones": ["+1 555-1234"]
    //     }
    // "#;

    // let deserialized_tool: ToolDefinition = serde_json::from_str(json_data)?;
    // println!("\nDeserialized Tool: {:?}", deserialized_tool);




    // 3. Working with untyped JSON (serde_json::Value)
    // This is useful when the structure of your JSON is not known at compile time.

    // let untyped_data = r#"
    //     {
    //         "product": "Laptop",
    //         "price": 1200.00,
    //         "features": ["SSD", "8GB RAM"]
    //     }
    // "#;

    // let value: Value = serde_json::from_str(untyped_data)?;
    // println!("\nUntyped JSON product: {}", value["product"]);
    // println!("Untyped JSON first feature: {}", value["features"][0]);

    Ok(())

}