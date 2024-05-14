#![feature(generic_const_exprs)]
use const_env::from_env;
use num_bigint::BigUint;
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng};
use rayon::prelude::*;
use std::{error::Error, time::Instant};

use summa_solvency_v1::{
    circuits::{
        merkle_sum_tree::MstInclusionCircuit,
        utils::{full_prover, generate_setup_artifacts},
        WithInstances,
    },
    merkle_sum_tree::{Cryptocurrency, Entry, MerkleSumTree, Tree},
};

use summa_bencher::BenchmarkResult;

#[from_env]
const K: u32 = 15;
#[from_env]
const LEVELS: usize = 17;
#[from_env]
const N_CURRENCIES: usize = 1;
#[from_env]
const N_BYTES: usize = 8;
#[from_env]
const N_USERS: usize = 1 << LEVELS;

pub fn generate_dummy_entries<const N_USERS: usize, const N_CURRENCIES: usize>(
) -> Result<Vec<Entry<N_CURRENCIES>>, Box<dyn Error>> {
    // Ensure N_CURRENCIES is greater than 0.
    if N_CURRENCIES == 0 {
        return Err("N_CURRENCIES must be greater than 0".into());
    }

    let mut entries: Vec<Entry<N_CURRENCIES>> = vec![Entry::zero_entry(); N_USERS];

    entries.par_iter_mut().for_each(|entry| {
        let mut rng = rand::thread_rng();

        let username: String = (0..10).map(|_| rng.sample(Alphanumeric) as char).collect();

        let balances: [BigUint; N_CURRENCIES] =
            std::array::from_fn(|_| BigUint::from(rng.gen_range(1000..90000) as u32));

        *entry = Entry::new(username, balances).expect("Failed to create entry");
    });

    Ok(entries)
}

fn main() {
    // Preprocessing
    // To generate commitment of v1, which is root hash of merkle sum tree
    let entries = generate_dummy_entries::<N_USERS, N_CURRENCIES>().unwrap();

    let cryptocurrencies: [Cryptocurrency; N_CURRENCIES] =
        std::array::from_fn(|_| Cryptocurrency {
            name: "ETH".to_string(),
            chain: "ETH".to_string(),
        });

    println!("Generating commitment proof(root has of MST) - v1");
    let build_mst_timer = Instant::now();
    let merkle_sum_tree = MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_entries(
        entries.clone(),
        cryptocurrencies.to_vec(),
        false,
    )
    .unwrap();
    let commitment_generation_time = build_mst_timer.elapsed();

    // Generate a random user index
    let random_user_index = OsRng.gen_range(0..N_USERS);

    let merkle_proof = merkle_sum_tree.generate_proof(random_user_index).unwrap();

    let circuit = MstInclusionCircuit::<LEVELS, N_CURRENCIES, N_BYTES>::init(merkle_proof);
    let (params, pk, _) = generate_setup_artifacts(K, None, circuit.clone()).unwrap();

    println!("Generating Inclusion proof - v1");
    let build_inclusion_timer = Instant::now();
    full_prover(&params, &pk, circuit.clone(), circuit.instances());
    let inclusion_proof_generation_time = build_inclusion_timer.elapsed();

    // Export output
    let benchmark_result = BenchmarkResult::new(
        K,
        N_USERS,
        N_CURRENCIES,
        commitment_generation_time.as_millis() as usize,
        inclusion_proof_generation_time.as_millis() as usize,
        "milliseconds".to_owned(),
    );

    println!("benchmark result: {:?}", benchmark_result);
    benchmark_result.save_as_file(&format!("v1_k{K}_u{N_USERS}_c{N_CURRENCIES}.json"));
}
