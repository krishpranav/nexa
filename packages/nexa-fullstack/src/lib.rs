use serde::{Deserialize, Serialize};

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
    use axum::{
        Json, Router,
        extract::{Multipart, Path, State},
        http::{HeaderMap, StatusCode},
        response::{
            IntoResponse, Response,
            sse::{Event, Sse},
        },
        routing::{get, post},
    };
    use once_cell::sync::Lazy;
    use std::collections::HashMap;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use tokio_stream::Stream;

    pub trait AuthContext: Send + Sync {
        fn check_auth(&self, headers: &HeaderMap) -> Result<(), ServerFnError>;
    }

    type ServerFnHandler = Arc<
        dyn Fn(
                serde_json::Value,
            )
                -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ServerFnError>> + Send>>
            + Send
            + Sync,
    >;

    static REGISTRY: Lazy<Arc<Mutex<HashMap<String, ServerFnHandler>>>> =
        Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

    pub fn register_server_fn(path: &str, handler: ServerFnHandler) {
        let mut registry = REGISTRY.lock().unwrap();
        registry.insert(path.to_string(), handler);
    }

    pub fn server_fn_router() -> Router {
        Router::new()
            .route("/api/:name", post(handle_server_fn))
            .route("/sse/:name", get(handle_sse))
    }

    pub async fn handle_server_fn(
        Path(name): Path<String>,
        headers: HeaderMap,
        body: axum::body::Bytes,
    ) -> impl IntoResponse {
        let handler = {
            let registry = REGISTRY.lock().unwrap();
            registry.get(&name).cloned()
        };

        if let Some(handler) = handler {
            let body_json: serde_json::Value = if headers
                .get("content-type")
                .map(|v| v == "application/cbor")
                .unwrap_or(false)
            {
                ciborium::from_reader(&body[..])
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()).into_response())
                    .unwrap()
            } else {
                serde_json::from_slice(&body)
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()).into_response())
                    .unwrap()
            };

            match handler(body_json).await {
                Ok(res) => {
                    if headers
                        .get("accept")
                        .map(|v| v == "application/cbor")
                        .unwrap_or(false)
                    {
                        let mut buf = Vec::new();
                        ciborium::into_writer(&res, &mut buf).unwrap();
                        Response::builder()
                            .header("content-type", "application/cbor")
                            .body(buf.into())
                            .unwrap()
                    } else {
                        Json(res).into_response()
                    }
                }
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.message).into_response(),
            }
        } else {
            (StatusCode::NOT_FOUND, "Function not found").into_response()
        }
    }

    pub async fn handle_sse(Path(name): Path<String>) -> impl IntoResponse {
        // Simple SSE stub
        let stream = tokio_stream::iter(vec![Ok::<_, std::convert::Infallible>(
            Event::default().data("ping"),
        )]);
        Sse::new(stream)
    }
}

pub mod client {

    #[cfg(target_arch = "wasm32")]
    pub mod wasm {
        use super::super::*;
        use gloo_net::http::Request;
        use wasm_bindgen::prelude::*;

        pub async fn call_server_fn<T: Serialize, R: for<'de> Deserialize<'de>>(
            path: &str,
            args: T,
            use_cbor: bool,
        ) -> Result<R, ServerFnError> {
            let mut req = Request::post(path);

            let resp = if use_cbor {
                let mut buf = Vec::new();
                ciborium::into_writer(&args, &mut buf).map_err(|e| ServerFnError {
                    message: e.to_string(),
                })?;
                req.header("content-type", "application/cbor")
                    .header("accept", "application/cbor")
                    .body(Some(&js_sys::Uint8Array::from(&buf[..])))
            } else {
                req.json(&args).map_err(|e| ServerFnError {
                    message: e.to_string(),
                })?
            }
            .send()
            .await
            .map_err(|e| ServerFnError {
                message: e.to_string(),
            })?;

            if !resp.ok() {
                return Err(ServerFnError {
                    message: resp.text().await.unwrap_or_default(),
                });
            }

            if use_cbor {
                let data = resp.binary().await.map_err(|e| ServerFnError {
                    message: e.to_string(),
                })?;
                ciborium::from_reader(&data[..]).map_err(|e| ServerFnError {
                    message: e.to_string(),
                })
            } else {
                resp.json().await.map_err(|e| ServerFnError {
                    message: e.to_string(),
                })
            }
        }
    }
}
