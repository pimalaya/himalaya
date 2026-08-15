---
cairn: change
change: first-run-wizard-frame
---

# Delta

## ADDED Requirements

### Requirement: A named command runs the wizard
A `configure` command (alias `wizard`) SHALL run the wizard by name, without the welcome, since whoever typed it knows what it does. It refuses to run when stdin is not a terminal, naming the sample configuration to write by hand instead.

### Requirement: The offer is a hook, not a gate
A missing configuration SHALL raise an offer to generate one, from a bare invocation and from any command needing an account. The offer never ends the process: a command carries on afterwards either way, so accepting gives it a chance to work and declining leaves it to fail on the configuration it still has not got. A bare invocation has nothing to carry on to, so a declined offer falls back to the help. Nothing is offered when stdin is not a terminal or `--json` is set.

### Requirement: The welcome names the missing path
The welcome SHALL name the configuration path that was looked for, which is the one `-c` or `HIMALAYA_CONFIG` gave or the default location, so a mistyped path shows up as itself rather than as a generic first run. It frames the product, points at the documented sample, and names the command that runs the wizard again later.

### Requirement: Generating never rewrites what a human wrote
The wizard SHALL write a configuration file that does not exist and append a plain text block to one that does, never parsing and re-serializing the document, so comments, ordering and formatting survive. Two invariants guard the append: the account name must be free, since a second `[accounts.<name>]` table makes the whole document fail to parse, and the generated account claims `default` only when no other account does. The derived name is suffixed until free. The target path is not prompted: it is where `-c` pointed, or the default location.

### Requirement: A generated account reads in a deliberate order
The serializer SHALL decide what a generated account holds, so a defaulted field is omitted and no field is enumerated twice, but the rendering SHALL order what it emits: the groups run most-defining first (`default`, the storage backend, the transport, the mailboxes, the rendering options), an unrecognised group renders after them rather than being dropped, a group's `server` key reads before the credentials qualifying it, and a blank line separates groups.

## MODIFIED Requirements

### Requirement: Account resolution failures name what is missing
Each of the three ways account resolution fails SHALL name what is missing and what to do about it: a missing configuration names the path it looked for, a missing named account lists the accounts the configuration does hold, and a missing default names both ways of picking one.

## REMOVED Requirements

### Requirement: The wizard prompts for its target path
The wizard no longer asks where to save. The path comes from `-c`, `HIMALAYA_CONFIG` or the default location, and an existing file is appended to rather than overwritten, so the overwrite confirmation goes with it.
