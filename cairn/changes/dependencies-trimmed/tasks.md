---
cairn: tasks
change: dependencies-trimmed
---

# Tasks

- [x] Cargo.toml: `default-features = false` and named features on every dependency; open, convert_case and uds_windows removed; toml a dev-dependency; pimalaya-cli `prompt` under `pimdir`; each feature checked against the item it gates.
- [x] base64 to base64-codec: src/shared/base64.rs (`encode`, `decode_strict`) behind `imap fetch`, the JMAP `Basic` header, `attachment download --stdout` and MCP `attach-data`; each call site tested, the tests run on the base64 crate first; `decode_strict` checked against the old decoder on every input of up to 7 symbols over 16 bytes and 200,000 encodings of random bytes, each also unpadded, over-padded and altered.
- [x] `SimdCpu::install_detected()` first in `main`.
- [x] `idna_adapter = "=1.0.0"`; `a_non_ascii_host_is_refused` (src/jmap/client.rs) seen failing on the ICU4X back end; IMAP, SMTP and ManageSieve hosts checked (opaque, never IDNA).
- [x] rfc2047-decoder replaced by `header::decode_bytes` in the IMAP envelope; `an_envelope_reads_its_header_strings_as_message_read_does` (src/imap/backend.rs) seen failing on the old decoder.
- [x] deny.toml: `[graph] targets`, licences trimmed to what the tree carries; `cargo deny check` warnings reported.
- [x] `JmapConfig::server` doc: `http` and `https` only.
- [x] fmt, clippy over the feature sets (three failing on dead code, as on ddcbaa4), `cargo test --all-features`, JSON schemas unchanged, crate count and release size before and after.
- [x] CHANGELOG; fold the delta into the spec (packaging, backends); log; status landed.
