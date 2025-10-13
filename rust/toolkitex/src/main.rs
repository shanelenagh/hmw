use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager
};
mod greeter;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
     //println!("Hello, world!");
    let service = StreamableHttpService::new(
        || Ok(greeter::Greeter::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let tcp_listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    let _ = axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await;
    Ok(())     
}
