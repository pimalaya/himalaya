---
cairn: change
change: shared-output-free-of-pimdir
---

# Delta

## ADDED Requirements

### Requirement: Shared outputs carry no backend details
The output of a shared command SHALL NOT carry a field that only one backend fills. A backend detail worth showing SHALL be logged by that backend's adapter or shown by its own namespace.

## MODIFIED Requirements

### Requirement: A queued creation is reported, not listed
A queued creation has no public id until the store's owner applies it, so the pimdir backend SHALL NOT project one as an envelope, and SHALL NOT put a placeholder in `Envelope.id`. `add_message` returns the link id it staged, which identifies the creation across the window. `himalaya pimdir queue list` is where queued creations show.

### Requirement: pimdir sends by queueing a submit intent
The command SHALL NOT report the queue row id in its output; the pimdir backend SHALL log it at info level.

### Requirement: The queue view shows queued sends
`himalaya pimdir queue list` SHALL render a queued `submit` as a message, derived from the body it pins as a create is, marked as a send, beside the queued creates of the same mailbox.
