---
cairn: delta
change: json-keys-are-camel-case
---

# Delta

Folds into cairn/spec/commands.md when 3.0 lands the rename, not before.

## ADDED Requirements

### Requirement: JSON keys are camelCase
Every output type handed to the printer SHALL serialize its keys as camelCase, matching the wire formats the backends speak (JMAP per RFC 8620, Microsoft Graph, the Google APIs) and keeping every key reachable by dot access in jq and JavaScript. A field carrying a provider spelling verbatim SHALL keep its explicit `#[serde(rename)]`, since `@odata.nextLink` and `nextPageToken` are the provider's names rather than derivable ones. Configuration types SHALL stay kebab-case: they are read from TOML and are not `--json` payloads.

## MODIFIED Requirements

## REMOVED Requirements
