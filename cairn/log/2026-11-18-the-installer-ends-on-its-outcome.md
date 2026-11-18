---
cairn: log
change: the-installer-ends-on-its-outcome
landed: 2026-11-18
---

# The installer ends on its outcome

Finding PKG-1 of the family's cli-family-analysis-2026-11-15 plan, with the owner's ruling of 2026-11-18 on the download flags.

## What landed

- install.sh: `die "…" 0` after the install, `curl -fsSLo` for the download.
- Checked by running the script's last lines against a stub binary: exit 127 before, 0 after.
- CHANGELOG (Fixed).

## Capabilities moved

- [packaging](../spec/packaging.md): *The installer ends on its outcome*, new.
