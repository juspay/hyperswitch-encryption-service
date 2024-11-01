// #![allow(clippy::expect_used)]
// #![allow(clippy::missing_panics_doc)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use josekit::jwe;



use base64::Engine;
use tokio::runtime::Runtime;
use rustc_hash::FxHashMap;

use cripta::{
    app::AppState,
    config,
    consts::base64::BASE64_ENGINE,
    core::crypto::custodian::{self, Custodian},
    types::{
        core::{DecryptedData, DecryptedDataGroup, Identifier},
        method::EncryptionType,
    },
    env::observability as logger
};

use std::sync::Arc;

// use cripta::{
//     crypto::aes::GcmAes256,
//     types::{DecryptedData, DecryptedDataGroup, Identifier},
//     app::AppState,  // Assuming you have an initialized AppState
//     custodian::Custodian,
// };

const ITERATION: u32 = 14;
// const JWE_PRIVATE_KEY: &str = include_str!("bench-private-key.pem");
// const JWE_PUBLIC_KEY: &str = include_str!("bench-public-key.pem");

criterion_main!(benches);
criterion_group!(benches, criterion_data_encryption_decryption);

pub fn criterion_data_encryption_decryption(c: &mut Criterion) {
    // let key = aes::generate_aes256_key();
    // let algo = aes::GcmAes256::new(key.to_vec());
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
        (1..ITERATION).for_each(|po| {
            let max: u64 = (2_u64).pow(po);
            let value = (0..max).map(|_| rand::random::<u8>()).collect::<Vec<_>>();

            logger::info!("Benchmarking for input: {:?}",value);
            let test_bs_data_clone = value.clone();
            // let bs_data = BASE64_ENGINE.decode("U2VjcmV0RGF0YQo=").unwrap();
            let test_data = DecryptedData::from_data(value.into());

            let dec_data = EncryptionType::Single(test_data);

// working encrypt fucntion
            // rt.block_on(async {
            //     let enc = dec_data.encrypt(&state, &identifier, custodian.clone()).await.expect("Failed while encrypting");
            //     return enc;
            // });
// working bench_function without groups

            // let val_bs = BASE64_ENGINE.encode(val2);
            // c.bench_function(&format!("Data Encryption Benchmark for input: {:?}",val_bs.len()), |b| {
            //     b.iter(|| {
            //         rt.block_on(async {
            //             let enc = dec_data.clone().encrypt(&state, &identifier.clone(), custodian.clone()).await.expect("Failed while encrypting");
            //             return enc;
            //         });
            //     })
            // });

            group.throughput(criterion::Throughput::Bytes(max));
            group.bench_with_input(
                criterion::BenchmarkId::from_parameter(max),
                &(test_bs_data_clone),
                |b, test_bs_data_clone| {
                    b.iter(|| {
                        black_box(
                            rt.block_on(async {
                                let enc = dec_data.clone().encrypt(&state, &identifier.clone(), custodian.clone()).await.expect("Failed while encrypting");
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
        (1..ITERATION).for_each(|po| {
            let max: u64 = (2_u64).pow(po);
            let value = (0..max).map(|_| rand::random::<u8>()).collect::<Vec<_>>();

            logger::info!("Benchmarking for input: {:?}",value);
            let test_bs_data_clone = value.clone();
            // let bs_data = BASE64_ENGINE.decode("U2VjcmV0RGF0YQo=").unwrap();
            let test_data = DecryptedData::from_data(value.into());

            let dec_data = EncryptionType::Single(test_data);

// working encrypt fucntion
            let encrypted_data = rt.block_on(async {
                let enc = dec_data.encrypt(&state, &identifier, custodian.clone()).await.expect("Failed while encrypting");
                return enc;
            });

            group_2.throughput(criterion::Throughput::Bytes(max));
            group_2.bench_with_input(
                criterion::BenchmarkId::from_parameter(max),
                &(test_bs_data_clone),
                |b, test_bs_data_clone| {
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

    // (1..ITERATION).for_each(|po| {
    //     let max: u64 = (2_u64).pow(po);
    //     let value = (0..max).map(|_| rand::random::<u8>()).collect::<Vec<_>>();
    //     let encrypted_value = algo
    //         .encrypt(value.clone())
    //         .expect("Failed while aes decrypting");
    //     group_2.throughput(criterion::Throughput::Bytes(max));
    //     group_2.bench_with_input(
    //         criterion::BenchmarkId::from_parameter(max),
    //         &(value, encrypted_value),
    //         |b, (value, encrypted_value)| {
    //             b.iter(|| {
    //                 black_box(
    //                     &algo
    //                         .decrypt(black_box(encrypted_value.clone()))
    //                         .expect("Failed while aes decrypting")
    //                         == value,
    //                 )
    //             })
    //         },
    //     );
    // });
}

// pub fn criterion_jwe_jws(c: &mut Criterion) {
//     let algo = jw::JWEncryption::new(
//         JWE_PRIVATE_KEY.to_string(),
//         JWE_PUBLIC_KEY.to_string(),
//         jwe::RSA_OAEP,
//         jwe::RSA_OAEP,
//     );

//     {
//         let mut group = c.benchmark_group("jw-encryption");
//         (1..ITERATION).for_each(|po| {
//             let max: u64 = (2_u64).pow(po);
//             let value = (0..max).map(|_| rand::random::<char>()).collect::<String>();
//             let value = value.as_bytes().to_vec();
//             let encrypted_value = algo
//                 .encrypt(value.clone())
//                 .expect("Failed while jw encrypting");
//             group.throughput(criterion::Throughput::Bytes(max));
//             group.bench_with_input(
//                 criterion::BenchmarkId::from_parameter(max),
//                 &(value, encrypted_value),
//                 |b, (value, encrypted_value)| {
//                     b.iter(|| {
//                         black_box(
//                             &algo
//                                 .encrypt(black_box(value.clone()))
//                                 .expect("Failed while jw encrypting")
//                                 == encrypted_value,
//                         )
//                     })
//                 },
//             );
//         });
//     }

//     let mut group_2 = c.benchmark_group("jw-decryption");
//     (1..ITERATION).for_each(|po| {
//         let max: u64 = (2_u64).pow(po);
//         let value = (0..max).map(|_| rand::random::<char>()).collect::<String>();
//         let value = value.as_bytes().to_vec();
//         let encrypted_value = algo
//             .encrypt(value.clone())
//             .expect("Failed while jw decrypting");
//         group_2.throughput(criterion::Throughput::Bytes(max));
//         group_2.bench_with_input(
//             criterion::BenchmarkId::from_parameter(max),
//             &(value, encrypted_value),
//             |b, (value, encrypted_value)| {
//                 b.iter(|| {
//                     black_box(
//                         &algo
//                             .decrypt(black_box(encrypted_value.clone()))
//                             .expect("Failed while jw decrypting")
//                             == value,
//                     )
//                 })
//             },
//         );
//     });
// }
