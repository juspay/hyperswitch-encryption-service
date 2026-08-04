use std::{
    task::{Context, Poll},
    time::Instant,
};

use axum::{
    extract::{MatchedPath, Request},
    response::Response,
};
use tower::{Layer, Service};

#[derive(Debug, Clone, Copy)]
pub struct HttpRequestMetricsLayer;

impl<S> Layer<S> for HttpRequestMetricsLayer {
    type Service = HttpRequestMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpRequestMetricsService { inner }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HttpRequestMetricsService<S> {
    inner: S,
}

// Using a drop guard ensures the counter is decremented on every exit path (normal response,
// early error, or panic), without requiring explicit cleanup at each branch.
struct ActiveRequestGuard {
    method: String,
    route: String,
}

impl ActiveRequestGuard {
    fn new(method: String, route: String) -> Self {
        super::HTTP_SERVER_ACTIVE_REQUESTS.add(
            1,
            metrics_utils::metric_attributes!(
                ("http.request.method", method.clone()),
                ("http.route", route.clone()),
            ),
        );

        Self { method, route }
    }
}

impl Drop for ActiveRequestGuard {
    fn drop(&mut self) {
        super::HTTP_SERVER_ACTIVE_REQUESTS.add(
            -1,
            metrics_utils::metric_attributes!(
                ("http.request.method", self.method.clone()),
                ("http.route", self.route.clone()),
            ),
        );
    }
}

impl<S> Service<Request> for HttpRequestMetricsService<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let route = req
            .extensions()
            .get::<MatchedPath>()
            .map(|p| p.as_str().to_owned())
            .unwrap_or_else(|| "UNKNOWN".to_string());

        super::HTTP_SERVER_REQUEST_COUNT.add(
            1,
            metrics_utils::metric_attributes!(
                ("http.request.method", method.clone()),
                ("http.route", route.clone()),
            ),
        );
        let active_request_guard = ActiveRequestGuard::new(method.clone(), route.clone());

        let future = self.inner.call(req);

        Box::pin(async move {
            let _active_request_guard = active_request_guard;
            let response = future.await?;
            let status = response.status().as_u16();
            let duration = start.elapsed();

            super::HTTP_SERVER_REQUEST_DURATION.record(
                duration.as_secs_f64(),
                metrics_utils::metric_attributes!(
                    ("http.request.method", method.clone()),
                    ("http.route", route.clone()),
                    ("http.response.status_code", status.to_string()),
                ),
            );

            Ok(response)
        })
    }
}
