use crate::errors::{self, ToContainerError};
use axum::Json;
use once_cell::sync::Lazy;
use opentelemetry::metrics::Histogram;
use std::time;

use opentelemetry::KeyValue;

pub(crate) async fn record_api_operation<F, T>(
    fut: F,
    metric: &Lazy<Histogram<f64>>,
    key_value: &[KeyValue],
) -> errors::ApiResponseResult<Json<T>>
where
    F: futures::Future<Output = errors::CustomResult<T, errors::ApplicationErrorResponse>>,
{
    let time = time::Instant::now();
    let result = fut.await.map(Json);
    let elapsed = time.elapsed();
    metric.record(elapsed.as_secs_f64(), key_value);
    result.to_container_error()
}
