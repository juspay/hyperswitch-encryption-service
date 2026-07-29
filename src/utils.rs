use std::{sync::LazyLock, time};

use axum::Json;
use metrics_utils::opentelemetry::{Histogram, KeyValue};

use crate::errors::{self, ToContainerError};

pub(crate) async fn record_api_operation<F, T>(
    fut: F,
    metric: &LazyLock<Histogram<f64>>,
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
