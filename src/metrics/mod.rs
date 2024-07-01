use once_cell::sync::Lazy;
use opentelemetry::metrics::{Counter, Histogram, Meter};

pub(crate) static METER: Lazy<Meter> = Lazy::new(|| opentelemetry::global::meter("cripta"));

pub(crate) static HEALTH_METRIC: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("HEALTH_METRIC").init());

pub(crate) static ENCRYPTION_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("ENCRYPTION_FAILURE").init());

pub(crate) static DECRYPTION_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("DECRYPTION_FAILURE").init());

pub(crate) static KEY_CREATE_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("KEY_CREATE_FAILURE").init());

pub(crate) static KEY_ROTATE_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("KEY_ROTATE_FAILURE").init());

pub(crate) static ENCRYPTION_API_LATENCY: Lazy<Histogram<f64>> =
    Lazy::new(|| METER.f64_histogram("ENCRYPTION_API_LATENCY").init());

pub(crate) static DECRYPTION_API_LATENCY: Lazy<Histogram<f64>> =
    Lazy::new(|| METER.f64_histogram("DECRYPTION_API_LATENCY").init());
