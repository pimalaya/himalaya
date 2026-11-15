---
cairn: delta
change: secret-command-resolved-once
---

# Delta

How many times a run reads a credential is user-visible, a key unlock being a prompt, so it is added to the config capability. The dependency bump and the `CommandConfig` type move no requirement: neither is something the spec describes.

## ADDED Requirements

### Requirement: A credential command is spawned once per run
A command backing a secret SHALL be spawned once per command run, however many blocks of the account name it, and its value SHALL be handed to every field naming it. A run that reaches IMAP, SMTP and ManageSieve therefore unlocks a `pass` or `gpg` entry once rather than three times.

Two commands SHALL count as one only where the configuration wrote them identically: the shell-string form and the argv-array form are distinct even when they run the same program.

The resolved value SHALL live no longer than the run that resolved it, and SHALL never be written to a config file, a log line or a cache.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
