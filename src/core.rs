pub mod crypto;
pub mod datakey;
mod health;

pub(crate) use crypto::*;
pub(crate) use datakey::*;
pub(crate) use health::*;

async fn record_domain_operation<Fut, T, E>(
    future: Fut,
    operation: crate::env::metrics::DomainOperation,
    data_identifier: String,
) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    use crate::env::metrics::{DOMAIN_OPERATION_COUNT, DOMAIN_OPERATION_DURATION};

    let start = std::time::Instant::now();
    let result = future.await;
    let duration = start.elapsed();
    let outcome = if result.is_ok() { "success" } else { "failure" };

    DOMAIN_OPERATION_COUNT.add(
        1,
        metrics_utils::metric_attributes!(
            ("operation", operation),
            ("outcome", outcome),
            ("data_identifier", data_identifier.clone()),
        ),
    );
    DOMAIN_OPERATION_DURATION.record(
        duration.as_secs_f64(),
        metrics_utils::metric_attributes!(
            ("operation", operation),
            ("outcome", outcome),
            ("data_identifier", data_identifier),
        ),
    );

    result
}
