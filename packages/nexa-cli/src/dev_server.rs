use anyhow::Result;
use axum::Router;
use log::info;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

pub async fn serve(port: u16) -> Result<()> {
    let app = Router::new().nest_service("/", ServeDir::new("dist"));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Nexa Dev Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
