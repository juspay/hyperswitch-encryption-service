#![allow(clippy::panic, clippy::expect_used)]

use axum::Router;
use cripta::{app::AppState, config, routes::*};
use router_env::logger;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let config = Arc::new(config::Config::with_config_path(
        config::Environment::Dev,
        None,
    ));

    let _guard = router_env::setup(
        &config.log,
        router_env::service_name!(),
        [router_env::service_name!(), "axum"],
    );

    logger::info!("Application started [{:?}] [{:?}]", config.server, config);

    let state = AppState::from_config(&config).await;

    let app = Router::new()
        .nest("/health", Health::server(state.clone()))
        .nest("/key", DataKey::server(state.clone()))
        .nest("/data", Crypto::server(state.clone()))
        .with_state(state);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", &config.server.host, &config.server.port))
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Unable to bind the {} and {}",
                    &config.server.host, &config.server.port
                )
            });
    axum::serve(listener, app)
        .await
        .expect("Unable to start the server");
}
