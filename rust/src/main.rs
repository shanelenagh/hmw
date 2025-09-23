use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};

// Define a struct that represents the structure of your JSON data.
// The `#[derive(Serialize, Deserialize)]` attributes from the `serde` crate
// automatically generate the necessary code for serialization and deserialization.
#[derive(Serialize, Deserialize, Debug)]
struct Person {
    name: String,
    age: u8,
    phones: Vec<String>,
}

fn main() -> Result<()>{
    //println!("Hello, world!");

    // Call an external program (e.g., 'ls' on Unix-like systems, 'dir' on Windows)
    let output = Command::new("ls") // Or "dir" on Windows
        .arg("-l") // Add an argument
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        println!("Command executed successfully!");
        println!("Output:\n{}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!("Command failed with error:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }

    // Spawning a process and waiting for it later
    let mut child = Command::new("sleep") // Or "timeout /t 5" on Windows
        .arg("5")
        .spawn()
        .expect("Failed to spawn child process");

    println!("Child process spawned, waiting for it to finish...");
    let status = child.wait().expect("Failed to wait on child process");
    println!("Child process exited with status: {:?}", status);
    
    
    /*
     * Now on to JSON demonstration
     */

    // 1. Serialization: Convert a Rust struct to a JSON string.
    let person = Person {
        name: "John Doe".to_string(),
        age: 43,
        phones: vec!["+44 1234567".to_string(), "+44 2345678".to_string()],
    };

    let json_string = serde_json::to_string_pretty(&person)?; // `to_string_pretty` for formatted output
    println!("Serialized JSON:\n{}", json_string);

    // 2. Deserialization: Convert a JSON string to a Rust struct.
    let json_data = r#"
        {
            "name": "Jane Smith",
            "age": 30,
            "phones": ["+1 555-1234"]
        }
    "#;

    let deserialized_person: Person = serde_json::from_str(json_data)?;
    println!("\nDeserialized Person: {:?}", deserialized_person);

    // 3. Working with untyped JSON (serde_json::Value)
    // This is useful when the structure of your JSON is not known at compile time.
    let untyped_data = r#"
        {
            "product": "Laptop",
            "price": 1200.00,
            "features": ["SSD", "8GB RAM"]
        }
    "#;

    let value: Value = serde_json::from_str(untyped_data)?;
    println!("\nUntyped JSON product: {}", value["product"]);
    println!("Untyped JSON first feature: {}", value["features"][0]);

    Ok(())

}