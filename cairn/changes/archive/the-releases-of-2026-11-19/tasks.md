---
cairn: tasks
change: the-releases-of-2026-11-19
---

# Tasks

- [x] Raise io-imap 0.12, io-smtp 0.10, io-sasl 0.3, io-managesieve 0.5, io-pimdir 0.18 and pimalaya-cli 0.7; `io-imap/std`, `io-smtp/std`; one io-sasl in `cargo tree`
- [x] HIM-18: `imap raw` from exact octets, `undecoded` `{base64}`, `split_responses` and the unframed fallback gone; tests `an_8bit_literal_prints_its_exact_octets` and `a_literal_line_spelling_a_completion_stays_in_its_literal` failing against the lossy reading first
- [x] C5: `is_mail`, `resolve_account`, `scope`, the coverage conditions and `overlay_quiet` on io-pimdir's helpers; test `a_configured_account_is_read_on_an_empty_store` failing on the old rule first (`store-account-unknown`)
- [x] `request-origin` in `api_url`, checked before the session; `a_url_off_the_api_is_refused` and `a_request_off_its_api_is_refused_before_the_session` failing first
- [x] `sanitize` on `pimalaya_cli::sanitize::path_component`
- [x] Spec folded (commands, backends, search), CHANGELOG, MIGRATION, log
