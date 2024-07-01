use opentelemetry::global;
use opentelemetry_sdk::{metrics::SdkMeterProvider, Resource};
use prometheus::default_registry;

pub(super) struct MetricsGuard {
    _metrics_guard: SdkMeterProvider,
}

#[allow(clippy::expect_used)]
pub(super) fn setup_metrics_pipeline() -> MetricsGuard {
    let registry = default_registry();

    let exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()
        .expect("Failed to build metrics pipeline");

    let resource = Resource::default();
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(exporter)
        .build();

    global::set_meter_provider(meter_provider.clone());

    MetricsGuard {
        _metrics_guard: meter_provider,
    }
}

#[allow(clippy::expect_used)]
impl Drop for MetricsGuard {
    fn drop(&mut self) {
        self._metrics_guard
            .shutdown()
            .expect("Failed to shutdown the metrics pipeline")
    }
}
