---
cairn: change
id: v2-release
status: active
created: 2026-07-25
---

# Cut the v2.0.0 release

## Why
The v2 product is feature-complete: io-email is gone, every backend is a per-protocol adapter over the io-* clients, and the wizard is finished. What stands between the current `2.0.0-alpha.1` and a tagged release is release plumbing, not missing features.

## What
Publish the dependency chain, drop the git patches, bump the version, and run a live provider pass.

The manifest currently patches io-imap, io-gmail, pimalaya-cli and pimalaya-config to git. Those crates must be published to crates.io at versions matching the declared requirements, then the `[patch.crates-io]` block removed. The pimalaya-cli change from the wizard work (the combined token picker with the `oauth` gate) must ship in the published cli before Himalaya can build against it, so cli leads the chain.

Then bump the version from alpha to the release, finalise the CHANGELOG `[Unreleased]` heading into a dated section, and run the manual provider reports under docs/testing against the published build.

## Out of scope
No behaviour change. This is packaging and verification only, so it carries no spec delta.
