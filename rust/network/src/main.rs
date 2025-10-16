use axum::{
    routing::get,
    Router,
};
use std::{
    collections::HashMap,
    sync::Mutex,
};
use lazy_static::lazy_static;
use uuid::Uuid;


#[derive(Debug)]
struct Session {
}

lazy_static! {
    static ref SESSION_MAP: Mutex<HashMap<String, Session>> = {
        Mutex::new(HashMap::new())
    };
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // Session map test
    let mut session_guard = SESSION_MAP.lock().unwrap();
    session_guard.insert(Uuid::new_v4().to_string(), Session {});
    println!("Session map contents: {:?}", *session_guard);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}