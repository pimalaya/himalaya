---
cairn: log
change: first-run-wizard-frame
landed: 2026-08-14
---

# Met the newcomer the way Comodoro does

Himalaya's wizard took Comodoro's frame. Discovery is untouched, so what an account ends up being is decided exactly as before; what changed is everything around that: how the wizard is reached, what it says on the way in, where it writes, and what happens when it is declined.

`himalaya configure` (alias `wizard`) now runs the wizard by name, so a second account no longer means a bare invocation, and declining the first-run offer is recoverable. The 1.x promise of a configure command is kept.

The offer became a hook. `resolve_account` used to prompt, run the wizard and call `exit(0)`, so the command someone typed never ran, whatever they answered: configuring successfully and then getting nothing is the same outcome as declining. It now offers, re-reads the configuration and carries on, so accepting gives the command a chance to work and declining leaves it to fail the ordinary way. The re-read matters because the wizard can also print the account instead of writing it, so having run it proves nothing landed.

Both entry points check `stdin().is_terminal()` and `printer.is_json()` first. A cron job or a `--json` caller hitting a missing configuration now gets an error it can read rather than a prompt nobody can answer, and `configure` itself refuses outright, naming the sample to write by hand.

The welcome moved onto the offer and names the path that was actually looked at, which is where `-c` or `HIMALAYA_CONFIG` pointed or the default location. Someone who mistyped a path sees the typo rather than a generic first-run banner. The command asked for by name skips the welcome.

Writing stopped prompting for a path and stopped being able to overwrite. The target comes from `Config::target_path`, a file that is not there is written whole, and one that is gets a plain text append, which is the only write that provably leaves a hand-written document alone. Two invariants guard it, both properties of the shared accounts table rather than of Himalaya: a duplicate `[accounts.<name>]` makes the whole file unparseable, and a second `default = true` makes the account every command picks depend on map ordering. The derived name is suffixed until free, which it has to be the moment appending exists.

The three resolution failures each name what is missing and what to do: the path that was read, the accounts the configuration does hold, and the two ways to pick a default.

Rendering gained an ordering pass. The serializer still decides what is written, so a defaulted field is omitted and nothing is enumerated twice, but the flattened dotted keys came out alphabetically: `imap.sasl.plain.authcid` sat above `imap.server`, burying the endpoint under the credentials that authenticate against it, and every group ran together. Groups now run most-defining first, `server` is lifted to the top of its own, and a blank line separates them. A group nobody listed renders after the known ones rather than being dropped, so a field added to `AccountConfig` can never go missing from a generated document because this table went stale.

`HIMALAYA_CONFIG` works again, through a `ConfigPathsArg` declared here rather than pimalaya-cli's shared `ConfigFlags`, since the variable has to carry the product name. Rewriting one arg per repository is cheaper than parameterising the shared one, and it is what Comodoro does. This needed clap's `env` feature, which Himalaya had not enabled.

Deliberately not done: discovery itself, which is specific to each product and is the half this change does not touch.

Verified: build, fmt and clippy clean; 93 tests pass, five of them new. The non-interactive paths were exercised end to end: a bare invocation with no configuration prints the help without prompting, `configure` refuses and names the sample, and the three resolution failures each print what they promise, including through `HIMALAYA_CONFIG`.

Spec updated: wizard (ADDED: A named command runs the wizard, The offer is a hook not a gate, The welcome names the missing path, Generating never rewrites what a human wrote, A generated account reads in a deliberate order; MODIFIED: Account resolution failures name what is missing; REMOVED: The wizard prompts for its target path).
