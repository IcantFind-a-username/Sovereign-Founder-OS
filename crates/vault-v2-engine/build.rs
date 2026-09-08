include!("build_gate.rs");

fn main() {
    for name in DEPENDENCY_SHAPING_VARIABLES {
        println!("cargo:rerun-if-env-changed={name}");
    }

    let environment: Vec<(String, String)> = std::env::vars().collect();
    let rejected = rejected_variables(&environment);
    if !rejected.is_empty() {
        for name in &rejected {
            println!("cargo:warning=refusing to build with {name} set");
        }
        panic!(
            "sovereign-vault-v2-engine: {} dependency-shaping environment override(s) are set \
             ({}). Build through scripts/qualify-vault-v2.sh, which constructs a clean child \
             environment, rather than clearing them by hand.",
            rejected.len(),
            rejected.join(", ")
        );
    }
}
