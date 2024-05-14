#![feature(generic_const_exprs)]
use const_env::from_env;
use halo2_proofs::{arithmetic::Field, halo2curves::bn256::Fr as Fp};
use num_bigint::BigUint;
use rand::{rngs::OsRng, Rng};
use std::time::Instant;

use summa_solvency_v2::circuits::univariate_grand_sum::UnivariateGrandSumConfig;
use summa_solvency_v2::{
    circuits::{
        univariate_grand_sum::UnivariateGrandSum,
        utils::{full_prover, generate_setup_artifacts, open_grand_sums, open_user_points},
    },
    utils::{big_uint_to_fp, generate_dummy_entries},
};

use summa_bencher::BenchmarkResult;

#[from_env]
const LEVELS: u32 = 17; // Used as K
#[from_env]
const N_CURRENCIES: usize = 1;
#[from_env]
const N_USERS: usize = (1 << LEVELS) - 6;

fn main() {
    // Preprocessing
    let entries = generate_dummy_entries::<N_USERS, N_CURRENCIES>().unwrap();

    // Calculate total for all entry columns
    let mut total_balances: Vec<BigUint> = vec![BigUint::from(0u32); N_CURRENCIES];

    for entry in &entries {
        for (i, balance) in entry.balances().iter().enumerate() {
            total_balances[i] += balance;
        }
    }

    let circuit_preperation_timer = Instant::now();
    let circuit = UnivariateGrandSum::<
        N_USERS,
        N_CURRENCIES,
        UnivariateGrandSumConfig<N_CURRENCIES, N_USERS>,
    >::init(entries.to_vec());
    let circuit_preperation_time = circuit_preperation_timer.elapsed();
    println!(
        "circuit preperation time: {:?}",
        circuit_preperation_time.as_millis()
    );

    let (params, pk, vk) = generate_setup_artifacts(LEVELS, None, &circuit).unwrap();

    // Generate commitment
    println!("Generating commitment proof(zk-SNARK proof + grans-sum proof) - v2");
    let build_commitment_timer = Instant::now();
    // Evaluatiing `zk-SNARK` proof with Univariate GransSum Circuit.
    let (_zk_snark_proof, advice_polys, _omega) =
        full_prover(&params, &pk, circuit, &[vec![Fp::zero()]]);

    let poly_length = 1 << u64::from(LEVELS);

    // Open first point for grand sum proof
    open_grand_sums(
        &advice_polys.advice_polys,
        &advice_polys.advice_blinds,
        &params,
        1..N_CURRENCIES + 1, // balance column range
        total_balances
            .iter()
            .map(|x| big_uint_to_fp(&(x)) * Fp::from(poly_length).invert().unwrap())
            .collect::<Vec<Fp>>()
            .as_slice(),
    );

    let commitment_generation_time = build_commitment_timer.elapsed();

    println!("Generating Inclusion proof - v2");
    let build_inclusion_timer = Instant::now();

    let column_range = 0..N_CURRENCIES + 1;
    let omega = vk.get_domain().get_omega();
    let random_user_index = OsRng.gen_range(0..N_USERS);
    let _openings_batch_proof = open_user_points(
        &advice_polys.advice_polys,
        &advice_polys.advice_blinds,
        &params,
        column_range.clone(),
        omega,
        random_user_index as u16,
        &entries
            .get(random_user_index as usize)
            .map(|entry| {
                std::iter::once(big_uint_to_fp(&(entry.username_as_big_uint())))
                    .chain(entry.balances().iter().map(|x| big_uint_to_fp(x)))
                    .collect::<Vec<Fp>>()
            })
            .unwrap(),
    );
    let inclusion_proof_generation_time = build_inclusion_timer.elapsed();

    let benchmark_result = BenchmarkResult::new(
        LEVELS,
        N_USERS,
        N_CURRENCIES,
        commitment_generation_time.as_millis() as usize,
        inclusion_proof_generation_time.as_millis() as usize,
        "milliseconds".to_owned(),
    );

    println!("benchmark result: {:?}", benchmark_result);
    benchmark_result.save_as_file(&format!("v2_k{LEVELS}_u{N_USERS}_c{N_CURRENCIES}.json"));
}
