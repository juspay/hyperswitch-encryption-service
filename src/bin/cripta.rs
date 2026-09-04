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

async fn spawn_management_server(
    host: &str,
    port: u16,
    prometheus_registry: Option<metrics_utils::prometheus::Registry>,
    state: Arc<AppState>,
) {
    use metrics_utils::prometheus::Encoder;

    #[expect(clippy::expect_used)]
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Unable to parse management server address");

    let mut app = Router::new();

    if let Some(registry) = prometheus_registry {
        app = app.route(
            "/metrics",
            axum::routing::get(move || {
                let registry = registry.clone();
                async move {
                    let encoder = metrics_utils::prometheus::TextEncoder::new();
                    let mut buffer = Vec::new();

                    if let Err(error) = encoder.encode(&registry.gather(), &mut buffer) {
                        tracing::warn!(?error, "Failed to encode prometheus metrics");
                    }

                    (
                        axum::http::StatusCode::OK,
                        String::from_utf8(buffer).unwrap_or_default(),
                    )
                }
            }),
        );
    }

    let app = app
        .nest("/health", Health::server(state.clone()))
        .with_state(state);

    #[expect(clippy::expect_used)]
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Unable to bind management server");

    tokio::spawn(async move {
        tracing::info!("Starting management server at `{addr}`");

        if let Err(error) = axum::serve(listener, app).await {
            tracing::warn!(?error, "Management server failed");
        }
    });
}

#[expect(clippy::expect_used)]
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

    let set_tcp_nodelay = config.server.set_tcp_nodelay;

    logger::info!(?config, "Application starting");

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

    let prometheus_registry = match guards.metrics_handle() {
        observability::MetricsHandle::Prometheus { inner } => inner.prometheus_registry().cloned(),
        _ => None,
    };

    // Spawn management server without mtls in a separate port
    spawn_management_server(
        &state.conf.management_server.host,
        state.conf.management_server.port,
        prometheus_registry,
        state.clone(),
    )
    .await;

    #[cfg(feature = "mtls")]
    {
        use axum_server::{
            accept::NoDelayAcceptor,
            tls_rustls::{RustlsAcceptor, RustlsConfig},
        };
        use cripta::app::tls;

        #[expect(clippy::panic)]
        let tls = tls::from_config(&state.conf)
            .await
            .unwrap_or_else(|err| panic!("unable to read the certificates. got err:{err:?}"));

        let tls_config = RustlsConfig::from_config(Arc::new(tls));

        if set_tcp_nodelay {
            // NoDelayAcceptor disables Nagle's algorithm on accepted sockets.
            axum_server::bind(host)
                .acceptor(RustlsAcceptor::new(tls_config).acceptor(NoDelayAcceptor::new()))
                .serve(app.into_make_service())
                .await
                .expect("unable to start the server")
        } else {
            axum_server::bind(host)
                .acceptor(RustlsAcceptor::new(tls_config))
                .serve(app.into_make_service())
                .await
                .expect("unable to start the server")
        }
    }

    #[cfg(not(feature = "mtls"))]
    {
        if set_tcp_nodelay {
            // NoDelayAcceptor disables Nagle's algorithm on accepted sockets.
            axum_server::bind(host)
                .acceptor(axum_server::accept::NoDelayAcceptor::new())
                .serve(app.into_make_service())
                .await
                .expect("unable to start the server")
        } else {
            axum_server::bind(host)
                .serve(app.into_make_service())
                .await
                .expect("unable to start the server")
        }
    }
}
