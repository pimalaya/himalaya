use std::env;

use git2::Repository;

fn main() {
    let repo = Repository::open(".").expect("should open git repository");
    let head = repo.head().expect("should get HEAD");
    let branch = head.shorthand().expect("should get branch name");
    let commit = head.peel_to_commit().expect("should get HEAD commit");
    let rev = commit.id().to_string();

    let version = env!("CARGO_PKG_VERSION");
    let os = env::var("CARGO_CFG_TARGET_OS").expect("should get CARGO_CFG_TARGET_OS");
    let env = env::var("CARGO_CFG_TARGET_ENV").expect("should get CARGO_CFG_TARGET_ENV");
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("should get CARGO_CFG_TARGET_ARCH");

    let long_version = [
        version,
        &os,
        &env,
        &arch,
        "git branch",
        &branch,
        "rev",
        &rev,
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .fold(String::new(), |mut version, section| {
        if !version.is_empty() {
            version.push(' ')
        }
        version.push_str(section);
        version
    });

    println!("cargo::rustc-env=HIMALAYA_LONG_VERSION={long_version}");
}
