---
cairn: delta
change: gmail-send-thread-id
---

# Delta

## ADDED Requirements

### Requirement: Gmail writes can target a thread
`gmail messages send`, `gmail drafts create` and `gmail drafts update` SHALL accept an optional `--thread-id <ID>`, sent as the message's `threadId`. Without it the request SHALL carry no thread id and Gmail chooses the thread.

#### Scenario: A reply sent into its thread
- GIVEN a Gmail thread `T` and a raw reply whose headers match it
- WHEN `himalaya gmail messages send --thread-id T` sends the reply
- THEN Gmail files the sent message in thread `T`
