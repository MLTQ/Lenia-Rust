// main.rs

use std::fs::File;
use std::io::{BufRead, BufReader};
use serde::Deserialize;
use toml;

mod visualizer;
mod lenia;

#[derive(Deserialize)]
struct Config {
    kernel: KernelConfig,
    visualizer: VisualizerConfig,
    files: FilesConfig,
}

#[derive(Deserialize)]
struct KernelConfig {
    size: usize,
    decay_constant: f64,
    decay_type: String,
    penalty_constant: f64,
}

#[derive(Deserialize)]
struct VisualizerConfig {
    refresh_rate: u64,
}

#[derive(Deserialize)]
struct FilesConfig {
    initial_state_file: String,
}



fn main() {
    // Read the config.toml file
    let config_str = std::fs::read_to_string("config.toml").expect("Unable to read config.toml");
    let config: Config = toml::from_str(&config_str).expect("Unable to parse config.toml");

    // Read the initial_state.csv file
    let file = File::open(&config.files.initial_state_file).expect("Unable to read initial_state.csv");
    let reader = BufReader::new(file);

    let initial_state: Vec<Vec<f64>> = reader
        .lines()
        .map(|line| {
            line.expect("Unable to read line")
                .split(',')
                .map(|s| s.parse::<f64>().expect("Unable to parse value as f64"))
                .collect()
        })
        .collect();

    // Run Lenia and visualize the result
    visualizer::run_visualization(
        config.kernel.size,
        config.kernel.decay_constant,
        config.kernel.decay_type,
        config.kernel.penalty_constant,
        config.visualizer.refresh_rate,
        initial_state,
    );
}
