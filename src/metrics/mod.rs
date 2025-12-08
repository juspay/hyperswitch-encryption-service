use once_cell::sync::Lazy;
use opentelemetry::metrics::{Counter, Histogram, Meter};

const fn duration_histogram_buckets() -> [f64; 25] {
    let mut buckets = [0.0; 25];
    let mut value = 1e-6; // 1 microsecond
    let mut i = 0;

    while i < 25 {
        buckets[i] = value;
        value *= 2.0;
        i += 1;
    }

    buckets
}

pub(crate) static METER: Lazy<Meter> = Lazy::new(|| opentelemetry::global::meter("cripta"));

pub(crate) static HEALTH_METRIC: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("HEALTH_METRIC").build());

pub(crate) static ENCRYPTION_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("ENCRYPTION_FAILURE").build());

pub(crate) static DECRYPTION_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("DECRYPTION_FAILURE").build());

pub(crate) static KEY_CREATE_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("KEY_CREATE_FAILURE").build());

pub(crate) static KEY_ROTATE_FAILURE: Lazy<Counter<u64>> =
    Lazy::new(|| METER.u64_counter("KEY_ROTATE_FAILURE").build());

pub(crate) static ENCRYPTION_API_LATENCY: Lazy<Histogram<f64>> = Lazy::new(|| {
    METER
        .f64_histogram("ENCRYPTION_API_LATENCY")
        .with_boundaries(Vec::from(duration_histogram_buckets()))
        .build()
});

pub(crate) static DECRYPTION_API_LATENCY: Lazy<Histogram<f64>> = Lazy::new(|| {
    METER
        .f64_histogram("DECRYPTION_API_LATENCY")
        .with_boundaries(Vec::from(duration_histogram_buckets()))
        .build()
});
