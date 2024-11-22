use std::env;

use git2::Repository;

fn main() {
    let branch = if let Ok(describe) = env::var("GIT_DESCRIBE") {
        describe
    } else {
        let repo = Repository::open(".").expect("should open git repository");
        let head = repo.head().expect("should get HEAD");
        head.shorthand()
            .expect("should get branch name")
            .to_string()
    };
    println!("cargo::rustc-env=GIT_DESCRIBE={branch}");

    let rev = if let Ok(rev) = env::var("GIT_REV") {
        rev
    } else {
        let repo = Repository::open(".").expect("should open git repository");
        let head = repo.head().expect("should get HEAD");
        let commit = head.peel_to_commit().expect("should get HEAD commit");
        commit.id().to_string()
    };
    println!("cargo::rustc-env=GIT_REV={rev}");

    let os = env::var("CARGO_CFG_TARGET_OS").expect("should get CARGO_CFG_TARGET_OS");
    println!("cargo::rustc-env=TARGET_OS={os}");

    let env = env::var("CARGO_CFG_TARGET_ENV").expect("should get CARGO_CFG_TARGET_ENV");
    println!("cargo::rustc-env=TARGET_ENV={env}");

    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("should get CARGO_CFG_TARGET_ARCH");
    println!("cargo::rustc-env=TARGET_ARCH={arch}");
}
