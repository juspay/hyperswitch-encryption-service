#![allow(clippy::panic, clippy::expect_used)]

use axum::Router;
use std::net::SocketAddr;

use cripta::{app::AppState, config, env::observability, env::observability as logger, routes::*};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let config = config::Config::with_config_path(config::Environment::which(), None);

    let _guard = observability::setup(&config.log, []);

    let host: SocketAddr = format!("{}:{}", &config.server.host, config.server.port)
        .parse()
        .expect("Unable to parse host");

    logger::info!("Application started [{:?}] [{:?}]", config.server, config);

    let state = Arc::new(AppState::from_config(config).await);

    let app = Router::new()
        .nest("/health", Health::server(state.clone()))
        .nest("/key", DataKey::server(state.clone()))
        .nest("/data", Crypto::server(state.clone()))
        .with_state(state.clone());

    // Spawn metrics server without mtls in a seperate port
    spawn_metrics_server(state.clone()).await;

    #[cfg(feature = "mtls")]
    {
        use axum_server::tls_rustls::RustlsConfig;
        use cripta::app::tls;

        let tls = tls::from_config(&state.conf)
            .await
            .unwrap_or_else(|err| panic!("unable to read the certificates. got err:{err:?}"));

        axum_server::bind_rustls(host, RustlsConfig::from_config(Arc::new(tls)))
            .serve(app.into_make_service())
            .await
            .expect("unable to start the server")
    }

    #[cfg(not(feature = "mtls"))]
    {
        axum_server::bind(host)
            .serve(app.into_make_service())
            .await
            .expect("unable to start the server")
    }
}

async fn spawn_metrics_server(state: Arc<AppState>) {
    let host: SocketAddr = format!(
        "{}:{}",
        &state.conf.metrics_server.host, &state.conf.metrics_server.port
    )
    .parse()
    .expect("Unable to parse metrics server");

    logger::info!(
        "Metrics Server started at [{:?}]",
        &state.conf.metrics_server
    );

    let app = Router::new()
        .nest("/metrics", Metrics::server(state.clone()))
        .with_state(state);

    axum_server::bind(host)
        .serve(app.into_make_service())
        .await
        .expect("Unable to start the metrics server")
}
