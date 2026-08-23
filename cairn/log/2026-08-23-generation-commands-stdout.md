---
cairn: log
change: generation-commands-stdout
landed: 2026-08-23
---

# Generation commands print to the standard output again

Restored the documented behaviour of `completion`, which printed a report and wrote files into the working directory instead of writing the completion script to stdout, breaking every packaging helper that captures it (#736). `manual` and `json-schema` were given the same shape, so the three meta commands behave alike: a positional list selects what to generate and defaults to everything, `--dir` decides where it lands, and without it the single selected item goes to stdout while asking for several fails. The three commands live in pimalaya-cli, released as 0.2.3, so nothing moved in this repository beyond the packaging derivation, which now passes its directory through `-d`. This moved the commands capability forward with a requirement covering the generation commands' output.
