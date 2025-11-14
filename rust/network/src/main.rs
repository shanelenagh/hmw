use argh::FromArgs;
use axum::{
    extract,
    routing::get, routing::post,
    Router
};
use lazy_static::lazy_static;
use mcpw_common::*;
use std::{
    error as std_error,
    result,
    collections::HashMap,
    sync::Mutex,
};
use tracing::{debug};
use uuid::Uuid;


#[derive(Debug)]
struct Session {
}

lazy_static! {
    static ref SESSION_MAP: Mutex<HashMap<String, Session>> = {
        Mutex::new(HashMap::new())
    };
}

/// TODO: Move to common lib !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
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

#[derive(Clone)]
struct AppState {
    tool_spec_map: HashMap<String, ToolDefinition>
}

#[tokio::main]
async fn main()  -> result::Result<(), Box<dyn std_error::Error>> {
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
    let Ok(tool_definitions) = serde_json::from_str::<Vec<ToolDefinition>>(&args.tool_specs) else {
        return Err(("Can't parse tool list (confirm schema with help CLI option): ".to_owned() + &args.tool_specs).into());
    };
    let state = AppState { 
        tool_spec_map: tool_definitions.iter()
            .map(|tool| (tool.mcp_tool_spec.name.clone(), tool.clone())).collect()
    };    
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/mcp", post(mcp_route))
        .with_state(state);

    // Session map test
    let mut session_guard = SESSION_MAP.lock().unwrap();
    session_guard.insert(Uuid::new_v4().to_string(), Session {});
    debug!("Session map contents: {:?}", *session_guard);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    return Ok(())
}

async fn mcp_route(extract::State(state): extract::State<AppState>, extract::Json(payload): extract::Json<JsonrpcRequest>) -> String {
    let string_payload = serde_json::to_string(&payload).unwrap();
    debug!("Received MCP data with method [{}] and full payload: {}", payload.method, &string_payload); 
    match payload.method.as_str() {
        "initialize" => {
            return mcp_init_string(payload.id, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).unwrap();
        },
        "tools/call" => {
            let Ok(tool_call_request) = serde_json::from_str::<CallToolRequest>(&string_payload) else {
                return jsonrpc_error_str(RequestId::from(-1), -32700, "Parsing of tool call request failed: ".to_string()+&string_payload).unwrap();
            };
            return match mcp_handle_tool_call(payload.id, &tool_call_request, &state.tool_spec_map) {
                Ok(response_str) => response_str,
                Err(err) => jsonrpc_error_str(RequestId::from(-1), -32603, "Tool call error: ".to_string()+&err.to_string()).unwrap()
            }   
        },        
        _ => {
            return jsonrpc_error_str(payload.id, -32601, "Method not found".to_string()).unwrap();
        }
    }
}