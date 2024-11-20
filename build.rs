use std::env;

use git2::Repository;

fn main() {
    if let Ok(repo) = Repository::open(".") {
        let head = repo.head().expect("should get git HEAD");
        let commit = head.peel_to_commit().expect("should get git HEAD commit");
        println!("cargo::rustc-env=GIT_REV={}", commit.id());
    }

    let os = env::var("CARGO_CFG_TARGET_OS").expect("should get CARGO_CFG_TARGET_OS");
    println!("cargo::rustc-env=TARGET_OS={os}");

    let env = env::var("CARGO_CFG_TARGET_ENV").expect("should get CARGO_CFG_TARGET_ENV");
    println!("cargo::rustc-env=TARGET_ENV={env}");

    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("should get CARGO_CFG_TARGET_ARCH");
    println!("cargo::rustc-env=TARGET_ARCH={arch}");
}
