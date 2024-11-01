use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use rustc_hash::FxHashMap;
use cripta::{
    app::AppState,
    config,
    core::crypto::custodian::Custodian,
    types::{
        core::{DecryptedData, DecryptedDataGroup, Identifier},
        method::EncryptionType,
    },
};

const SINGLE_BENCH_ITERATION: u32 = 10;
const BATCH_BENCH_ITERATION: u32 = 10;

criterion_main!(benches);
criterion_group!(benches, criterion_data_encryption_decryption, criterion_batch_data_encryption_decryption);

pub fn criterion_data_encryption_decryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let custodian = Custodian::new(Some(("key".to_string(),"value".to_string())));
    let config = config::Config::with_config_path(config::Environment::which(), None);
    let state = rt.block_on(async {
         let state = AppState::from_config(config).await;
         return state;
    });
    let identifier = Identifier::User(String::from("user_12345"));

    {
        let mut group = c.benchmark_group("data-encryption-single");
        (1..SINGLE_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let value = (0..input_size).map(|_| rand::random::<u8>()).collect::<Vec<_>>();
            let test_bs_data_clone = value.clone();
            let bench_input = EncryptionType::Single(DecryptedData::from_data(value.into()));
            group.throughput(criterion::Throughput::Bytes(input_size));
            group.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(test_bs_data_clone),
                |b, _test_bs_data_clone| {
                    b.iter(|| {
                        black_box(
                            rt.block_on(async {
                                let enc = bench_input.clone().encrypt(&state, &identifier.clone(), custodian.clone()).await.expect("Failed while encrypting");
                                return enc;
                            })
                        )
                    })
                },
            );
        });
    }
    {
        let mut group_2 = c.benchmark_group("data-decryption-single");
        (1..SINGLE_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let value = (0..input_size).map(|_| rand::random::<u8>()).collect::<Vec<_>>();
            let test_bs_data_clone = value.clone();
            let bench_input = EncryptionType::Single(DecryptedData::from_data(value.into()));
            let encrypted_data = rt.block_on(async {
                let enc = bench_input.encrypt(&state, &identifier, custodian.clone()).await.expect("Failed while encrypting");
                return enc;
            });

            group_2.throughput(criterion::Throughput::Bytes(input_size));
            group_2.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(test_bs_data_clone),
                |b, _test_bs_data_clone| {
                    b.iter(|| {
                        black_box(
                            rt.block_on(async {
                                let enc = encrypted_data.clone().decrypt(&state, &identifier.clone(), custodian.clone()).await.expect("Failed while encrypting");
                                return enc;
                            })
                        )
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
        let value = DecryptedData::from_data((0..1024).map(|_| rand::random::<u8>()).collect::<Vec<_>>().into());
        batch_map.insert(key, value);
    }

    DecryptedDataGroup(batch_map)
}


pub fn criterion_batch_data_encryption_decryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let custodian = Custodian::new(Some(("key".to_string(),"value".to_string())));
    let config = config::Config::with_config_path(config::Environment::which(), None);
    let state = rt.block_on(async {
         let state = AppState::from_config(config).await;
         return state;
    });
    let identifier = Identifier::User(String::from("user_12345"));
    {
        let mut group = c.benchmark_group("data-encryption-batch");
        (1..BATCH_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let bench_input = EncryptionType::Batch(generate_batch_data(input_size));
            group.throughput(criterion::Throughput::Bytes(input_size));
            group.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(bench_input),
                |b, _bench_input| {
                    b.iter(|| {
                        black_box(
                            rt.block_on(async {
                                let enc = bench_input.clone().encrypt(&state, &identifier.clone(), custodian.clone()).await.expect("Failed while encrypting");
                                return enc;
                            })
                        )
                    })
                },
            );
        });
    }
    {
        let mut group_2 = c.benchmark_group("data-decryption-batch");
        (1..BATCH_BENCH_ITERATION).for_each(|po| {
            let input_size: u64 = (2_u64).pow(po);
            let decrypted_input = EncryptionType::Batch(generate_batch_data(input_size));
            let encrypted_bench_input = rt.block_on(async {
                let enc = decrypted_input.encrypt(&state, &identifier, custodian.clone()).await.expect("Failed while encrypting");
                return enc;
            });
            group_2.throughput(criterion::Throughput::Bytes(input_size));
            group_2.bench_with_input(
                criterion::BenchmarkId::from_parameter(input_size),
                &(encrypted_bench_input),
                |b, _encrypted_bench_input| {
                    b.iter(|| {
                        black_box(
                            rt.block_on(async {
                                let enc = encrypted_bench_input.clone().decrypt(&state, &identifier.clone(), custodian.clone()).await.expect("Failed while encrypting");
                                return enc;
                            })
                        )
                    })
                },
            );
        });
    }

}