use std::{
    collections::HashMap,
    env::{self, VarError},
};

use git2::{DescribeOptions, Repository};
use serde::Deserialize;

fn main() {
    features_env();
    target_envs();
    git_envs();
}

/// Builds the `CARGO_FEATURES` environment variable.
///
/// This function turns enabled cargo features into a simple string
/// `+feature1 +feature2 +featureN`, which then exposes it via the
/// `CARGO_FEATURES` environment variable.
///
/// It first reads and parses the Cargo.toml in order to extract all
/// available features (omitting "default"). It then checks for
/// enabled features via `CARGO_FEATURE_<name>` to finally collect
/// them into a string.
fn features_env() {
    #[derive(Deserialize)]
    struct Config {
        features: HashMap<String, Vec<String>>,
    }

    impl Config {
        fn enabled_features(self) -> impl Iterator<Item = String> {
            self.features
                .into_keys()
                .filter(|feature| feature != "default")
                .filter(|feature| {
                    let feature = feature.replace('-', "_").to_uppercase();
                    env::var(format!("CARGO_FEATURE_{feature}")).is_ok()
                })
        }
    }

    let config: Config =
        toml::from_str(include_str!("./Cargo.toml")).expect("should parse Cargo.toml");

    let mut features = String::new();

    for feature in config.enabled_features() {
        if !features.is_empty() {
            features.push(' ');
        }
        features.push_str(&format!("+{feature}"));
    }

    println!("cargo::rustc-env=CARGO_FEATURES={features}");
}

/// Builds environment variables related to the target platform.
///
/// This function basically forwards existing cargo environments
/// related to the target platform.
fn target_envs() {
    forward_env("CARGO_CFG_TARGET_OS");
    forward_env("CARGO_CFG_TARGET_ENV");
    forward_env("CARGO_CFG_TARGET_ARCH");
}

/// Builds environment variables related to git.
///
/// This function basically tries to forward existing git environment
/// variables. In case of failure, it tries to build them using
/// [`git2`].
fn git_envs() {
    let git = Repository::open(".").ok();

    if try_forward_env("GIT_DESCRIBE").is_err() {
        let description = match &git {
            None => String::from("unknown"),
            Some(git) => {
                let mut opts = DescribeOptions::new();
                opts.describe_all();
                opts.show_commit_oid_as_fallback(true);

                git.describe(&opts)
                    .expect("should describe git object")
                    .format(None)
                    .expect("should format git object description")
            }
        };

        println!("cargo::rustc-env=GIT_DESCRIBE={description}");
    };

    if try_forward_env("GIT_REV").is_err() {
        let rev = match &git {
            None => String::from("unknown"),
            Some(git) => {
                let head = git.head().expect("should get git HEAD");
                let commit = head.peel_to_commit().expect("should get git HEAD commit");
                commit.id().to_string()
            }
        };

        println!("cargo::rustc-env=GIT_REV={rev}");
    };
}

/// Tries to forward the given environment variable.
///
/// For a more strict version, see [`forward_env`].
fn try_forward_env(key: &str) -> Result<String, VarError> {
    let env = env::var(key);

    if let Ok(val) = &env {
        println!("cargo::rustc-env={key}={val}");
    }

    env
}

/// Forwards the given environment variable.
///
/// This function panics in case the forward fails (when the
/// environment variable does not exist for example).
///
/// For a less strict version, see [`try_forward_env`].
fn forward_env(key: &str) {
    try_forward_env(key).expect(&format!("should get env {key}"));
}
