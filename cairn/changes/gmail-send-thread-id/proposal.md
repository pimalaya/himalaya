---
cairn: change
id: gmail-send-thread-id
status: landed
created: 2026-10-09
---

# Gmail messages send can join a thread

## Why

`users.messages.send` accepts a `threadId`, and Gmail files the sent message into that thread only when it is given (and the subject and references headers agree). `gmail drafts create` and `gmail drafts update` already expose it as `--thread-id`, but `gmail messages send` does not, so a reply sent directly lands in a new thread on the sender's side.

## What

- `gmail messages send` takes an optional `--thread-id <ID>`, passed as the message's `threadId`. Without it the request is unchanged.
