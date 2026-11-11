---
cairn: change
id: first-run-wizard-frame
status: landed
created: 2026-08-14
---

# Adopt Comodoro's first-run wizard frame

## Why

Comodoro's `first-run-wizard` landed the shape a Pimalaya CLI should meet a newcomer with, and Himalaya has most of the pieces but wired differently. The parts that differ are the ones that decide whether someone who just installed the binary gets somewhere.

Himalaya has no named wizard command. The wizard runs only from a bare `himalaya`, so nothing generates a second account and nothing re-runs it after a decline. The 1.x promise of a `configure` command is unkept.

The offer is a gate rather than a hook: `resolve_account` prompts, runs the wizard and calls `exit(0)`, so the command someone actually typed never runs, whatever they answered. Comodoro's carries on either way, so accepting gives the command a chance to work and declining leaves it to fail on the configuration it still has not got.

Nothing guards interactivity on the way in. A cron job or a `--json` caller hitting a missing configuration gets a prompt it cannot answer instead of an error it can read.

The welcome names no path. Someone who mistyped `-c` sees a generic first-run banner rather than the path that was actually looked at, which is the one fact that would tell them what went wrong.

An existing configuration can only be overwritten, never appended to, so generating a second account means the user merges it by hand. Comodoro appends as plain text and guards the two invariants the account map otherwise breaks silently.

Two of the three resolution failures say too little: a missing named account does not list the accounts that do exist, and a missing default does not name both ways of picking one.

## What

The wizard keeps its discovery flow unchanged; what changes is the frame around it.

A `configure` command (alias `wizard`) runs the wizard by name, with no welcome, since whoever typed it knows what it does. The welcome belongs to the offer, and names the configuration path that was looked for.

The offer becomes a hook raised from the two places nothing can happen without a configuration: a bare `himalaya`, and any command needing an account. It never exits: a command carries on afterwards either way, and a bare invocation, having nothing to carry on to, falls back to the help.

Nothing prompts when stdin is not a terminal or `--json` is set, in the offer and in `configure` alike.

The target path stops being prompted and comes from `Config::target_path`, which is where `-c` pointed or the default location. A configuration already there is appended to as plain text rather than overwritten, under the two rules Comodoro established: the account name must be free, since a second `[accounts.<name>]` table makes the whole document unparseable, and the generated account claims `default` only when no other one does.

The three resolution failures each name what is missing and what to do: the path that was read, the accounts the configuration holds, and the two ways to pick a default.

## Scope / non-goals

Discovery is untouched. The email, URL and folder-path inputs, the per-protocol flows, the connection tests and the mailbox aliases all stay exactly as they are. This change is the wizard's frame, not its content.

Account naming stays derived rather than prompted, but gains Comodoro's suffix-until-free loop, which it needs the moment appending exists.

The rendering gains an ordering pass. The serializer still decides what is written, so no field is listed twice and none can go missing, but the flattened dotted keys come out alphabetically, which buries `imap.server` under the credentials authenticating against it and runs every group together. The groups are reordered most-defining-first, `server` is lifted to the top of its own, and a blank line separates them.

`HIMALAYA_CONFIG` comes back, through a `ConfigPathsArg` declared in himalaya rather than pimalaya-cli's shared `ConfigFlags`, since the variable has to carry the product name. Rewriting one arg per repository is cheaper than parameterising the shared one.
