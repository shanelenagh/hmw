use argh::FromArgs;
use axum::{
    extract,
    routing::post,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    Router
};
use mcpw_common::*;
use std::{
    error as std_error,
    result,
    collections::HashMap
};
use tower_http::cors::{CorsLayer, Any};
use tracing::{debug};
//use uuid::Uuid;


// TODO: Fill out and use this for MCP sessions
#[derive(Debug, Clone)]
struct Session {
}

/// TODO: Move to common lib !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
#[derive(FromArgs, Clone)]
struct Args {
    #[argh(option, short='t', description="array of tool specification command wrapper mappings in JSON format: [ {{ \"command\": \"scriptOrExecutable\", <\"command_parameters\": [ <\"mcp_param\": \"nameOfMcpMethodArgParameterToMapToCommandParam\">, <\"command_param\": \"staticCommandSwitchOrSwitchForMcpParameter\" ]>, \"mcp_tool_spec\": {{ mcpToolSpecJsonPerOfficialMcpSchema... }} }} , ... ]")]
    tool_specs: String, 
    #[cfg(feature = "debug_log")]
    #[argh(switch, short='d', description="debug output on stderr (will show up in console of MCP server/inspector)")]
    debug: bool,
    #[cfg(feature = "debug_log")]
    #[argh(switch, short='p', description="pretty print log (including console ASCII coloring)")]
    pretty: bool,    
    #[argh(switch, short='s', description="use sessions (via Mcp-Session-Id header)")]
    use_session: bool  
}

#[derive(Clone)]
struct AppState {
    args: Args,
    tool_spec_map: HashMap<String, ToolDefinition>,
    tools: Vec<Tool>,
    sessions: HashMap<String, Session>
}


#[tokio::main]
async fn main()  -> result::Result<(), Box<dyn std_error::Error>> {
    let args: Args = argh::from_env();
    conditionally_enable_debugging(&args);
    let Ok(tool_definitions) = serde_json::from_str::<Vec<ToolDefinition>>(&args.tool_specs) else {
        return Err(("Can't parse tool list (confirm schema with help CLI option): ".to_owned() + &args.tool_specs).into());
    };
    let state = AppState { 
        args: args,
        tool_spec_map: tool_definitions.iter().map(|tool| (tool.mcp_tool_spec.name.clone(), tool.clone())).collect(),
        tools: tool_definitions.iter().map(|tool| tool.mcp_tool_spec.clone()).collect::<Vec<Tool>>(),
        sessions: HashMap::new()
    };    
    // build our application with a single route
    let app = Router::new()
        .route("/mcp", post(mcp_route))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(Any));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    return Ok(())
}

async fn mcp_route(extract::State(mut state): extract::State<AppState>, extract::Json(payload): extract::Json<JsonrpcRequest>) 
    -> axum::response::Response 
{
    let string_payload = serde_json::to_string(&payload).unwrap();
    debug!("Received MCP data with method [{}] and full payload: {}", payload.method, &string_payload); 
    match payload.method.as_str() {
        "initialize" => {
            let mut headers = HeaderMap::new();
            if state.args.use_session {
                let session_id = uuid::Uuid::new_v4().to_string();
                headers.insert("Mcp-Session-Id", session_id.parse().unwrap());
                state.sessions.insert(session_id, Session{});
            } 
            return (StatusCode::OK, headers, Json(mcp_init(payload.id, 
                env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))).into_response();
        },
        "tools/call" => {
            let Ok(tool_call_request) = serde_json::from_str::<CallToolRequest>(&string_payload) else {
                return (StatusCode::UNPROCESSABLE_ENTITY, Json(jsonrpc_error(RequestId::from(-1), -32700, 
                    "Parsing of tool call request failed: ".to_string()+&string_payload))).into_response();
            };
            return match mcp_handle_tool_call(payload.id, &tool_call_request, &state.tool_spec_map) {
                Ok(response) => (StatusCode::OK, Json(response)).into_response(),
                Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
            }   
        },  
        "tools/list" => {
            return (StatusCode::OK, Json(mcp_tools_list(payload.id, &state.tools))).into_response();
        },              
        _ => {
            return (StatusCode::UNPROCESSABLE_ENTITY, Json(jsonrpc_error(payload.id, -32601, 
                "Method not found: ".to_string() + payload.method.as_str()))).into_response();
        }
    }
}

fn conditionally_enable_debugging(args: &Args) {
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