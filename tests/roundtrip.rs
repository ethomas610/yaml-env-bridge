use std::fs;
use std::path::Path;

use yaml_env_bridge::{env_to_yaml, yaml_to_env};

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading fixture {:?}: {}", path, e))
}

fn to_env(yaml: &str) -> String {
    let mut out = Vec::new();
    yaml_to_env::convert(yaml.as_bytes(), &mut out).expect("yaml_to_env::convert");
    String::from_utf8(out).expect("yaml_to_env output was not utf8")
}

fn to_yaml(env: &str) -> String {
    let mut out = Vec::new();
    env_to_yaml::convert(env.as_bytes(), &mut out).expect("env_to_yaml::convert");
    String::from_utf8(out).expect("env_to_yaml output was not utf8")
}

/// Checks both conversion directions against a matched pair of fixtures,
/// and that converting there and back lands exactly where it started - the
/// fixtures are written so both are true at once, since each is already the
/// canonical form the other direction would produce.
fn check_pair(yaml_fixture: &str, env_fixture: &str) {
    let yaml = fixture(yaml_fixture);
    let env = fixture(env_fixture);

    assert_eq!(to_env(&yaml), env, "yaml -> env mismatch for {}", yaml_fixture);
    assert_eq!(to_yaml(&env), yaml, "env -> yaml mismatch for {}", env_fixture);

    assert_eq!(to_yaml(&to_env(&yaml)), yaml, "yaml round trip mismatch for {}", yaml_fixture);
    assert_eq!(to_env(&to_yaml(&env)), env, "env round trip mismatch for {}", env_fixture);
}

#[test]
fn round_trips_simple_mapping() {
    check_pair("simple.yaml", "simple.env");
}

#[test]
fn round_trips_sequences_of_mappings_and_scalars() {
    check_pair("sequences.yaml", "sequences.env");
}

#[test]
fn round_trips_root_level_sequence() {
    check_pair("root_sequence.yaml", "root_sequence.env");
}
