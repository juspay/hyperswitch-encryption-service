#![allow(clippy::panic, clippy::expect_used)]

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, body::Body};
use cripta::{
    app::AppState,
    config,
    consts::{TENANT_HEADER, X_REQUEST_ID},
    env::{observability, observability as logger},
    request_id::MakeUlid,
    routes::*,
};
use hyper::Request;
use tower::ServiceBuilder;
use tower_http::{ServiceBuilderExt, trace::TraceLayer};

#[tokio::main]
async fn main() {
    let config = config::Config::with_config_path(config::Environment::which(), None);
    config.validate();

    let _guard = observability::setup(&config.log, [], env!("CARGO_BIN_NAME"));

    let host: SocketAddr = format!("{}:{}", &config.server.host, config.server.port)
        .parse()
        .expect("Unable to parse host");

    logger::info!(?config, "Application starting");

    #[cfg(any(feature = "mtls", feature = "postgres_ssl"))]
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("unable to install default crypto provider");

    let state = Arc::new(AppState::from_config(config).await);

    let middleware = ServiceBuilder::new()
        .set_x_request_id(MakeUlid)
        .propagate_x_request_id()
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let tenant_id = request.headers().get(TENANT_HEADER).and_then(|r| r.to_str().ok()).unwrap_or("invalid_tenant");
                let request_id = request.headers().get(X_REQUEST_ID).and_then(|r| r.to_str().ok()).unwrap_or("unknown_id");

                tracing::debug_span!("request",request_id = %request_id,method = %request.method(), uri=%request.uri(), tenant_id=%tenant_id)
            })
            .on_request(logger::OnRequest::with_level(logger::LogLevel::Info))
            .on_response(logger::OnResponse::with_level(logger::LogLevel::Info))
        );

    let app = Router::new()
        .nest("/health", Health::server(state.clone()))
        .nest("/key", DataKey::server(state.clone()))
        .nest("/data", Crypto::server(state.clone()))
        .layer(middleware)
        .with_state(state.clone());

    // Spawn metrics server without mtls in a seperate port
    tokio::task::spawn(spawn_metrics_server(state.clone()));

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
        .nest("/health", Health::server(state.clone()))
        .nest("/metrics", Metrics::server(state.clone()))
        .with_state(state);

    axum_server::bind(host)
        .serve(app.into_make_service())
        .await
        .expect("Unable to start the metrics server")
}
