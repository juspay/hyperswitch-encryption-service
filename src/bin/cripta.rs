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
        .with_state(state.clone());

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
