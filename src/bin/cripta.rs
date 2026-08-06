#![allow(clippy::panic, clippy::expect_used)]

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, body::Body};
use cripta::{
    app::AppState,
    config,
    consts::{TENANT_HEADER, X_REQUEST_ID},
    env::{observability, observability as logger},
    request_id::MakeUuidV7,
    routes::*,
};
use hyper::Request;
use tower::ServiceBuilder;
use tower_http::{ServiceBuilderExt, trace as tower_trace};

#[cfg(feature = "vergen")]
fn default_headers() -> tower_http::set_header::SetResponseHeaderLayer<axum::http::HeaderValue> {
    tower_http::set_header::SetResponseHeaderLayer::overriding(
        axum::http::HeaderName::from_static("x-version"),
        axum::http::HeaderValue::from_static(build_info::git_describe!()),
    )
}

#[tokio::main]
async fn main() {
    let config = config::Config::with_config_path(config::Environment::which(), None);
    config.validate();

    let guards = observability::setup(
        &config,
        [env!("CARGO_BIN_NAME"), "tower_http"],
        env!("CARGO_BIN_NAME"),
    )
    .expect("Failed to initialize observability");

    let host: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .expect("Unable to parse host");

    logger::info!(?config, "Application starting");

    let background_metrics_interval = config.metrics.background_metrics_collection_interval_secs();

    #[cfg(any(feature = "mtls", feature = "postgres_ssl"))]
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("unable to install default crypto provider");

    let state = Arc::new(AppState::from_config(config).await);

    let middleware = ServiceBuilder::new()
        .set_x_request_id(MakeUuidV7)
            .option_layer({
                #[cfg(feature = "vergen")]
                {
                    Some(default_headers())
                }
                #[cfg(not(feature = "vergen"))]
                {
                    None::<tower_http::set_header::SetResponseHeaderLayer<axum::http::HeaderValue>>
                }
            })
        .layer(
            tower_trace::TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
                let tenant_id = request.headers().get(TENANT_HEADER).and_then(|r| r.to_str().ok()).unwrap_or("invalid_tenant");
                let request_id = request.headers().get(X_REQUEST_ID).and_then(|r| r.to_str().ok()).unwrap_or("unknown_id");

                tracing::debug_span!("request", request_id = %request_id, method = %request.method(), uri = %request.uri(), tenant_id = %tenant_id)
            })
            .on_request(tower_trace::DefaultOnRequest::new().level(tracing::Level::INFO))
            .on_response(
                tower_trace::DefaultOnResponse::new()
                    .level(tracing::Level::INFO)
                    .latency_unit(tower_http::LatencyUnit::Micros),
            )
            .on_failure(
                tower_trace::DefaultOnFailure::new()
                    .latency_unit(tower_http::LatencyUnit::Micros)
                    .level(tracing::Level::ERROR),
            )
        )
        .propagate_x_request_id()
        .option_layer(
            guards
                .metrics_handle()
                .is_enabled()
                .then_some(observability::HttpRequestMetricsLayer),
        );

    let app = Router::new()
        .nest("/health", Health::server(state.clone()))
        .nest("/key", DataKey::server(state.clone()))
        .nest("/data", Crypto::server(state.clone()))
        .layer(middleware)
        .with_state(state.clone());

    if let observability::MetricsHandle::Prometheus { inner, host, port } = guards.metrics_handle()
        && let Some(registry) = inner.prometheus_registry()
    {
        // Spawn metrics server without mtls in a seperate port
        observability::spawn_prometheus_metrics_server(
            host,
            *port,
            registry.clone(),
            state.clone(),
        )
        .expect("Failed to start Prometheus metrics server");
    }

    if guards.metrics_handle().is_enabled() {
        observability::spawn_bg_metrics_collector(&state, background_metrics_interval);
    }

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
