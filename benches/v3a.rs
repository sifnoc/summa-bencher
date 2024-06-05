use const_env::from_env;
use plonkish_backend::{
    backend::{hyperplonk::HyperPlonk, PlonkishBackend, PlonkishCircuit, PlonkishCircuitInfo},
    frontend::halo2::Halo2Circuit,
    halo2_curves::bn256::{Bn256, Fr as Fp},
    pcs::{multilinear::MultilinearKzg, Evaluation, PolynomialCommitmentScheme},
    util::{transcript::{InMemoryTranscript, Keccak256Transcript}},
};
use rand::{rngs::OsRng, Rng};
use std::time::Instant;

use summa_bencher::{BenchmarkResult, seeded_std_rng};
use summa_solvency_v3a::{
    circuits::summa_circuit::summa_hyperplonk::SummaHyperplonk,
    utils::{big_uint_to_fp, generate_dummy_entries, uni_to_multivar_binary_index},
};

#[from_env]
const LEVELS: u32 = 17;
#[from_env]
const N_CURRENCIES: usize = 1;
#[from_env]
const N_USERS: usize = (1 << LEVELS) - 1;

fn main() {
    type ProvingBackend = HyperPlonk<MultilinearKzg<Bn256>>;
    let entries = generate_dummy_entries::<N_USERS, N_CURRENCIES>().unwrap();
    let halo2_circuit =
        SummaHyperplonk::<N_USERS, N_CURRENCIES>::init(
            entries.to_vec(),
        );

    let circuit = Halo2Circuit::<
        Fp,
        SummaHyperplonk<N_USERS, N_CURRENCIES>,
    >::new::<ProvingBackend>(LEVELS as usize, halo2_circuit.clone());

    let circuit_info: PlonkishCircuitInfo<_> = circuit.circuit_info().unwrap();
    let param = ProvingBackend::setup(&circuit_info, seeded_std_rng()).unwrap();

    let (pp, vp) = ProvingBackend::preprocess(&param, &circuit_info).unwrap();

    println!("Generating commitment proof(grand-sum proof) - v3a");
    let build_commitment_timer = Instant::now();
    let mut transcript = Keccak256Transcript::default();
    let witness_polys = ProvingBackend::prove(&pp, &circuit, &mut transcript, seeded_std_rng()).unwrap();
    let proof = transcript.into_proof();
    let commitment_generation_time = build_commitment_timer.elapsed();

    let num_points = N_CURRENCIES + 1;
    let user_entry_polynomials = witness_polys.iter().take(num_points).collect::<Vec<_>>();

    let mut transcript = Keccak256Transcript::from_proof((), proof.as_slice());

    let user_entry_commitments =
        MultilinearKzg::<Bn256>::read_commitments(&vp.pcs, num_points, &mut transcript).unwrap();

    let random_user_index = OsRng.gen_range(0..entries.len());
    let multivariate_challenge = uni_to_multivar_binary_index(&random_user_index, LEVELS as usize);

    let mut evals = vec![];
    for i in 0..N_CURRENCIES + 1 {
        if i == 0 {
            evals.push(Evaluation::new(
                i,
                0,
                big_uint_to_fp::<Fp>(entries[random_user_index].username_as_big_uint()),
            ));
        } else {
            evals.push(Evaluation::new(
                i,
                0,
                big_uint_to_fp::<Fp>(&entries[random_user_index].balances()[i - 1]),
            ));
        }
    }

    println!("Generating Inclusion proof - v3a");
    let build_inclusion_timer = Instant::now();
    let mut kzg_transcript = Keccak256Transcript::new(());
    MultilinearKzg::<Bn256>::batch_open(
        &pp.pcs,
        user_entry_polynomials,
        &user_entry_commitments,
        &[multivariate_challenge],
        &evals,
        &mut kzg_transcript,
    )
    .unwrap();
    let inclusion_proof_generation_time = build_inclusion_timer.elapsed();

    // Export output
    let benchmark_result = BenchmarkResult::new(
        LEVELS,
        N_USERS,
        N_CURRENCIES,
        commitment_generation_time.as_millis() as usize,
        inclusion_proof_generation_time.as_millis() as usize,
        "milliseconds".to_owned(),
    );

    println!("benchmark result: {:?}", benchmark_result);
    benchmark_result.save_as_file(&format!("v3a_k{LEVELS}_u{N_USERS}_c{N_CURRENCIES}.json"));
}
