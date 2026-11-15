---
cairn: delta
change: a-request-stays-on-its-api
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: The Gmail and Graph escape hatches are requests
When built with the `gmail` or `msgraph` feature, `gmail request <METHOD> <PATH> [JSON]` and `msgraph request <METHOD> <PATH> [JSON]` SHALL send one HTTP request under the account's bearer token, as Cardamum's `gpeople request` and `msgraph request` do: METHOD `get`, `post`, `patch` or `delete`, `put` too on Gmail, in any case, another refused before connecting; PATH joined onto `https://gmail.googleapis.com/gmail/v1/` or `https://graph.microsoft.com/v1.0/`, a leading `/` dropped, or a full URL taken as it is when it stays on that API's origin (`https://gmail.googleapis.com`, `https://graph.microsoft.com`), a URL off it refused before any request, the bearer token never leaving for another host; JSON the body of a `post`, `put` or `patch`, `null` when absent. They SHALL print `{response}`, the body the provider answered as it sent it, `null` for an empty one, its text form the body pretty-printed, nothing for an empty one. A refusal SHALL fail as the typed commands' refusals do.

#### Scenario: A Graph patch
- GIVEN a Microsoft Graph account
- WHEN `himalaya --json msgraph request patch /me/mailFolders/F1 '{"displayName":"Work"}'` runs
- THEN it sends `PATCH /v1.0/me/mailFolders/F1` with that body and prints `{"response": <the folder Graph answered>}`


## REMOVED Requirements

None.
