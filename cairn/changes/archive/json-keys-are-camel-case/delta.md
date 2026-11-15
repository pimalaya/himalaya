---
cairn: delta
change: json-keys-are-camel-case
---

# Delta

Folds into cairn/spec/commands.md.

## ADDED Requirements

### Requirement: JSON keys are camelCase
Every output type handed to the printer SHALL serialize its keys as camelCase, matching the wire formats the backends speak (JMAP per RFC 8620, Microsoft Graph, the Google APIs) and keeping every key reachable by dot access in jq and JavaScript. Each one SHALL be named `<Domain><Target><Verb>Output`, the domain omitted for the shared commands. A field carrying a provider spelling verbatim SHALL keep its explicit `#[serde(rename)]`, since `@odata.nextLink` and `nextPageToken` are the provider's names rather than derivable ones, and a transparent newtype over an io-gmail or io-msgraph resource SHALL serialize that resource as its crate declares it. The rule covers keys only: string values keep their own spelling (error codes, flags, mailbox roles, `message delete` actions and pimdir queue states stay kebab-case or lowercase). Configuration types SHALL stay kebab-case: they are read from TOML and are not `--json` payloads.

## MODIFIED Requirements

## REMOVED Requirements
