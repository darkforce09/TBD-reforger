use super::*;

fn owns(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|s| (*s).to_string()).collect()
}

mod scope_mapping_tests;
