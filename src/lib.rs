use serde::Serialize;
use serde_json::to_string_pretty;
use std::{fs::File, io::Write};

#[derive(Serialize, Debug)]
pub struct BenchmarkResult {
    k: u32,
    n_users: usize,
    n_currencies: usize,
    commitment_generation_time: usize,
    inclusion_generation_time: usize,
    time_unit: String,
}

impl BenchmarkResult {
    // Initialize result
    pub fn new(
        k: u32,
        n_users: usize,
        n_currencies: usize,
        commitment_generation_time: usize,
        inclusion_generation_time: usize,
        time_unit: String,
    ) -> Self {
        Self {
            k,
            n_users,
            n_currencies,
            commitment_generation_time,
            inclusion_generation_time,
            time_unit,
        }
    }

    pub fn save_as_file(&self, filename: &str) {
        // Serialize to a JSON string
        let serialized_data = to_string_pretty(self).expect("Failed to benchmark  data");

        // Save the serialized data to a JSON file
        let mut file = File::create(filename).expect("Unable to create file");
        file.write_all(serialized_data.as_bytes())
            .expect("Unable to write data to file");
    }
}
