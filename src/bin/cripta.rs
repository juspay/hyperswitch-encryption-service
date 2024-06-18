#![allow(clippy::panic, clippy::expect_used)]

use axum::Router;
use std::net::SocketAddr;

use cripta::{app::AppState, config, routes::*};
use router_env::logger;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let config = config::Config::with_config_path(config::Environment::which(), None);

    let _guard = router_env::setup(
        &config.log,
        router_env::service_name!(),
        [router_env::service_name!(), "axum"],
    );

    let host: SocketAddr = format!("{}:{}", &config.server.host, config.server.port)
        .parse()
        .expect("Unable to parse host");

    logger::info!("Application started [{:?}] [{:?}]", config.server, config);

    let state = Arc::new(AppState::from_config(config).await);

    let app = Router::new()
        .nest("/health", Health::server(state.clone()))
        .nest("/key", DataKey::server(state.clone()))
        .nest("/data", Crypto::server(state.clone()))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&host)
        .await
        .unwrap_or_else(|_| panic!("Unable to bind the {}", &host));

    axum::serve(listener, app)
        .await
        .expect("Unable to start the server");
}
