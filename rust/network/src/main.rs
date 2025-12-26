use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post
};
use mcpw_common::*;
use serde_json::{from_str, to_string};
use std::collections::HashMap;
use tokio::{net::TcpListener, main};
use tower_http::cors::{CorsLayer, Any};
use tracing::{debug};
use uuid::Uuid;


// TODO: Placeholder struct for persisting data in session map (now only ID is created as key, and this is empty value)
#[derive(Debug, Clone)]
struct Session {
}

/// Common state used by all route handlers
#[derive(Clone)]
struct AppState {
    args: Args,
    tool_spec_map: HashMap<String, ToolDefinition>,
    tools: Vec<Tool>,
    sessions: HashMap<String, Session>
}


#[main]
async fn main()  -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = get_args();
    conditionally_enable_debugging(&args);
    let Ok(tool_definitions) = from_str::<Vec<ToolDefinition>>(&args.tool_specs) else {
        return Err(("Can't parse tool list (confirm schema with help CLI option): ".to_owned() + &args.tool_specs).into());
    };
    let state = AppState { 
        args: args.clone(),
        tool_spec_map: tool_definitions.iter().map(|tool| (tool.mcp_tool_spec.name.clone(), tool.clone())).collect(),
        tools: tool_definitions.iter().map(|tool| tool.mcp_tool_spec.clone()).collect::<Vec<Tool>>(),
        sessions: HashMap::new()
    };    
    // build our application with a single route
    let app = axum::Router::new()
        .route("/mcp", post(mcp_route))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(Any)); // TODO: Make CORS configurable
    debug!("Starting MCP network server on {}:{}", &args.host, &args.port);
    axum::serve(TcpListener::bind(args.host + ":" + &args.port.to_string()).await.unwrap(), app).await.unwrap();
    return Ok(())
}

async fn mcp_route(State(mut state): State<AppState>, Json(payload): Json<JsonrpcRequest>) -> Response 
{
    let string_payload = to_string(&payload).unwrap();
    debug!("Received MCP data with method [{}] and full payload: {}", payload.method, &string_payload); 
    match payload.method.as_str() {
        "initialize" => {
            let mut headers = HeaderMap::new();
            if state.args.use_session {
                let session_id = Uuid::new_v4().to_string();
                headers.insert("Mcp-Session-Id", session_id.parse().unwrap());
                state.sessions.insert(session_id, Session{});
            } 
            return (StatusCode::OK, headers, axum::Json(mcp_init(payload.id))).into_response();
        },
        "tools/call" => {
            let Ok(tool_call_request) = from_str::<CallToolRequest>(&string_payload) else {
                return (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(jsonrpc_error(RequestId::from(-1), -32700, 
                    "Parsing of tool call request failed: ".to_string()+&string_payload))).into_response();
            };
            return match mcp_handle_tool_call(payload.id, &tool_call_request, &state.tool_spec_map) {
                Ok(response) => (StatusCode::OK, axum::Json(response)).into_response(),
                Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(err)).into_response()
            }   
        },  
        "tools/list" => {
            return (StatusCode::OK, axum::Json(mcp_tools_list(payload.id, &state.tools))).into_response();
        },              
        _ => {
            return (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(jsonrpc_error(payload.id, -32601, 
                "Method not found: ".to_string() + payload.method.as_str()))).into_response();
        }
    }
}