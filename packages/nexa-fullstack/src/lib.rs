use serde::{Serialize, Deserialize};


pub use nexa_fullstack_macro::server;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerFnError {
    pub message: String,
}

impl std::fmt::Display for ServerFnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ServerFnError: {}", self.message)
    }
}

impl std::error::Error for ServerFnError {}

#[cfg(feature = "ssr")]
pub mod server {
    use super::*;
    use std::future::Future;
    use axum::{
        routing::post,
        Router,
        Json,
        extract::Path,
        response::{IntoResponse, Response},
        http::StatusCode,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use once_cell::sync::Lazy;
    use std::pin::Pin;

    // Type for a generic server function handler
    // Takes raw JSON body, returns JSON response
    type ServerFnHandler = Arc<dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ServerFnError>> + Send>> + Send + Sync>;

    // Global registry of server functions
    static REGISTRY: Lazy<Arc<Mutex<HashMap<String, ServerFnHandler>>>> = Lazy::new(|| {
        Arc::new(Mutex::new(HashMap::new()))
    });
    
    pub fn register_server_fn(path: &str, handler: ServerFnHandler) {
        let mut registry = REGISTRY.lock().unwrap();
        registry.insert(path.to_string(), handler);
    }
    
    pub fn server_fn_router() -> Router {
        Router::new().route("/api/:name", post(handle_server_fn))
    }
    
    pub async fn handle_server_fn(Path(name): Path<String>, body: axum::body::Bytes) -> impl IntoResponse {
        let handler = {
            let registry = REGISTRY.lock().unwrap();
            registry.get(&name).cloned()
        };

        if let Some(handler) = handler {
            let body_json: serde_json::Value = match serde_json::from_slice(&body) {
                Ok(v) => v,
                Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            };
            
            match handler(body_json).await {
                Ok(res) => Json(res).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.message).into_response(),
            }
        } else {
            (StatusCode::NOT_FOUND, "Function not found").into_response()
        }
    }
}

#[cfg(not(feature = "ssr"))]
pub mod client {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    pub async fn call_server_fn<T: Serialize, R: for<'de> Deserialize<'de>>(
        path: &str,
        args: T
    ) -> Result<R, ServerFnError> {
        use gloo_net::http::Request;
        let resp = Request::post(path)
            .json(&args)
            .map_err(|e| ServerFnError { message: e.to_string() })?
            .send()
            .await
            .map_err(|e| ServerFnError { message: e.to_string() })?;
            
        if !resp.ok() {
            return Err(ServerFnError { 
                message: resp.text().await.unwrap_or_else(|_| "Unknown error".to_string()) 
            });
        }
        
        resp.json().await.map_err(|e| ServerFnError { message: e.to_string() })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn call_server_fn<T: Serialize, R: for<'de> Deserialize<'de>>(
        _path: &str,
        _args: T
    ) -> Result<R, ServerFnError> {
        Err(ServerFnError { message: "Client server functions not implemented for non-WASM targets yet".to_string() })
    }
}
