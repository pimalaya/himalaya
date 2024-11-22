use std::{collections::HashMap, env};

use git2::{DescribeOptions, Repository};
use serde::Deserialize;

fn main() {
    if let Ok(git) = Repository::open(".") {
        if let None = maybe_forward_env("GIT_DESCRIBE") {
            let mut opts = DescribeOptions::new();
            opts.describe_all();
            opts.show_commit_oid_as_fallback(true);

            let description = git
                .describe(&opts)
                .expect("should describe git object")
                .format(None)
                .expect("should format git object description");

            println!("cargo::rustc-env=GIT_DESCRIBE={description}");
        };

        if let None = maybe_forward_env("GIT_REV") {
            let head = git.head().expect("should get git HEAD");
            let commit = head.peel_to_commit().expect("should get git HEAD commit");
            let rev = commit.id().to_string();

            println!("cargo::rustc-env=GIT_REV={rev}");
        };
    }

    let toml: CargoToml =
        toml::from_str(include_str!("./Cargo.toml")).expect("should read Cargo.toml");

    let mut features = String::new();

    for (feature, _) in toml.features {
        if feature == "default" {
            continue;
        }

        if feature_enabled(&feature) {
            features.push(' ');
            features.push_str(&format!("+{feature}"));
        }
    }

    println!("cargo::rustc-env=CARGO_FEATURES={features}");

    forward_env("CARGO_CFG_TARGET_OS");
    forward_env("CARGO_CFG_TARGET_ENV");
    forward_env("CARGO_CFG_TARGET_ARCH");
}

#[derive(Deserialize)]
struct CargoToml {
    features: HashMap<String, Vec<String>>,
}

fn feature_enabled(feature: &str) -> bool {
    let feature = feature.replace('-', "_").to_uppercase();
    env::var(format!("CARGO_FEATURE_{feature}")).is_ok()
}

fn maybe_forward_env(key: &str) -> Option<String> {
    match env::var(key) {
        Err(_) => None,
        Ok(val) => {
            println!("cargo::rustc-env={key}={val}");
            Some(val)
        }
    }
}

fn forward_env(key: &str) {
    maybe_forward_env(key).expect(&format!("should get env {key}"));
}
