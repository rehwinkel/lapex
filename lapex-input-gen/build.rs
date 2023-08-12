use std::env;
use std::path::Path;

use lapex::{generate, Language, ParsingAlgorithm};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_lapex");
    std::fs::create_dir_all(&dest_path).unwrap();
    generate(
        true,
        ParsingAlgorithm::LR1,
        false,
        Path::new("src/lapex.lapex"),
        &dest_path,
        Language::Rust,
        lapex_input_bootstrap::BootstrapLapexInputParser {},
    );
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lapex.lapex");
}
