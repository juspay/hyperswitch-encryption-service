use std::sync::Arc;

use cripta::{
    app::AppState,
    config,
    core::{crypto::custodian::Custodian, datakey::create::generate_and_create_data_key},
    types::{
        core::{DecryptedData, DecryptedDataGroup, Identifier},
        method::EncryptionType,
        requests::CreateDataKeyRequest,
    },
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustc_hash::FxHashMap;
use tokio::runtime::Runtime;

// Command: cargo bench
// Note: modify this to run for different size inputs
const SINGLE_BENCH_ITERATION: u32 = 10;
const BATCH_BENCH_ITERATION: u32 = 10;

criterion_main!(benches);
criterion_group!(
    benches,
    criterion_data_encryption_decryption,
    criterion_batch_data_encryption_decryption
);

/// # Panics
///
/// Panics if failed to build thread pool
#[allow(clippy::expect_used)]
pub fn criterion_data_encryption_decryption(c: &mut Criterion) {
    let rt = Runtime::new().expect("error in runTime creation");
    let custodian = Custodian::new(Some(("key".to_string(), "value".to_string())));
    let config = config::Config::with_config_path(config::Environment::Dev, None);
    let state = rt.block_on(async { AppState::from_config(config).await });
    // create a DataKey in data_key_store
    let config2 = config::Config::with_config_path(config::Environment::Dev, None);
    let identifier = Identifier::User(String::from("bench_user"));
    let key_create_req: CreateDataKeyRequest = CreateDataKeyRequest {
        identifier: identifier.clone(),
    };
    let key_create_state = rt.block_on(async {
        let state = AppState::from_config(config2).await;
        <AppState as Into<Arc<AppState>>>::into(state)
    });
    rt.block_on(async {
        let _ =
            generate_and_create_data_key(key_create_state, custodian.clone(), key_create_req).await;
    });

    {
        let mut group = c.benchmark_group("data-encryption-single");
        (0..SINGLE_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let value = (0..input_size)
                .map(|_| rand::random::<u8>())
                .collect::<Vec<_>>();
            let test_bs_data_clone = value.clone();
            let bench_input = EncryptionType::Single(DecryptedData::from_data(value.into()));
            group.throughput(criterion::Throughput::Bytes(input_size));
            group.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(test_bs_data_clone),
                |b, _test_bs_data_clone| {
                    b.iter(|| {
                        black_box(rt.block_on(async {
                            bench_input
                                .clone()
                                .encrypt(&state, &identifier.clone(), custodian.clone())
                                .await
                                .expect("Failed while encrypting")
                        }))
                    })
                },
            );
        });
    }
    {
        let mut group_2 = c.benchmark_group("data-decryption-single");
        (0..SINGLE_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let value = (0..input_size)
                .map(|_| rand::random::<u8>())
                .collect::<Vec<_>>();
            let test_bs_data_clone = value.clone();
            let bench_input = EncryptionType::Single(DecryptedData::from_data(value.into()));
            let encrypted_data = rt.block_on(async {
                bench_input
                    .encrypt(&state, &identifier, custodian.clone())
                    .await
                    .expect("Failed while encrypting")
            });

            group_2.throughput(criterion::Throughput::Bytes(input_size));
            group_2.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(test_bs_data_clone),
                |b, _test_bs_data_clone| {
                    b.iter(|| {
                        black_box(rt.block_on(async {
                            encrypted_data
                                .clone()
                                .decrypt(&state, &identifier.clone(), custodian.clone())
                                .await
                                .expect("Failed while decrypting")
                        }))
                    })
                },
            );
        });
    }
}

fn generate_batch_data(size: u64) -> DecryptedDataGroup {
    let mut batch_map = FxHashMap::default();
    for i in 0..size {
        let key = format!("key_{}", i);
        let value = DecryptedData::from_data(
            (0..1024)
                .map(|_| rand::random::<u8>())
                .collect::<Vec<_>>()
                .into(),
        );
        batch_map.insert(key, value);
    }

    DecryptedDataGroup(batch_map)
}

/// # Panics
///
/// Panics if failed to build thread pool
#[allow(clippy::expect_used)]
pub fn criterion_batch_data_encryption_decryption(c: &mut Criterion) {
    let rt = Runtime::new().expect("error in runTime creation");
    let custodian = Custodian::new(Some(("key".to_string(), "value".to_string())));
    let config = config::Config::with_config_path(config::Environment::Dev, None);
    let state = rt.block_on(async { AppState::from_config(config).await });
    let identifier = Identifier::User(String::from("bench_user"));
    {
        let mut group = c.benchmark_group("data-encryption-batch");
        (0..BATCH_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let bench_input = EncryptionType::Batch(generate_batch_data(input_size));
            group.throughput(criterion::Throughput::Bytes(input_size));
            group.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(bench_input),
                |b, _bench_input| {
                    b.iter(|| {
                        black_box(rt.block_on(async {
                            bench_input
                                .clone()
                                .encrypt(&state, &identifier.clone(), custodian.clone())
                                .await
                                .expect("Failed while encrypting")
                        }))
                    })
                },
            );
        });
    }
    {
        let mut group_2 = c.benchmark_group("data-decryption-batch");
        (0..BATCH_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let decrypted_input = EncryptionType::Batch(generate_batch_data(input_size));
            let encrypted_bench_input = rt.block_on(async {
                decrypted_input
                    .encrypt(&state, &identifier, custodian.clone())
                    .await
                    .expect("Failed while encrypting")
            });
            group_2.throughput(criterion::Throughput::Bytes(input_size));
            group_2.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(encrypted_bench_input),
                |b, _encrypted_bench_input| {
                    b.iter(|| {
                        black_box(rt.block_on(async {
                            encrypted_bench_input
                                .clone()
                                .decrypt(&state, &identifier.clone(), custodian.clone())
                                .await
                                .expect("Failed while decrypting")
                        }))
                    })
                },
            );
        });
    }
}
