#!/usr/bin/env bash
#
# cairn verify: check this repository against the Cairn conformance rules
# (CAIRN.md section 8). Strictly read-only.
#
# Vendored from github.com/pimalaya/cairn (impl/bash), flattened into one
# file so the repository needs nothing checked out beside it. Optional: the
# same nine checks can be run by reading, which is what AGENTS.md asks of an
# agent.
#
#   cairn/verify.sh [dir] [--block]     dir defaults to the current directory

#
# Shared helpers for the Cairn bash reference port. Sourced, not executed.
# This port is OPTIONAL. Cairn needs no tooling. See ../../GUIDE.md.

die() { printf 'cairn: %s\n' "$*" >&2; exit 2; }

# Walk up from $1 to the nearest ancestor holding a `cairn/` dir or `cairn.toml`.
find_root() {
  local dir; dir=$(cd "$1" 2>/dev/null && pwd) || return 1
  while :; do
    if [ -d "$dir/cairn" ] || [ -f "$dir/cairn.toml" ]; then
      printf '%s\n' "$dir"; return 0
    fi
    [ "$dir" = "/" ] && return 1
    dir=$(dirname "$dir")
  done
}

# Print the leading YAML frontmatter block, between the first two `---`.
# Emits nothing when the file does not open with `---`.
fm_block() {
  awk 'NR==1 && $0 != "---" { exit }
       /^---[[:space:]]*$/  { c++; if (c==2) exit; next }
       c==1 { print }' "$1" 2>/dev/null
}

# Value of frontmatter key $2 in file $1. Inline `# comments` are stripped.
fm_get() {
  fm_block "$1" | sed -n "s/^$2:[[:space:]]*//p" | head -n1 \
    | sed -e 's/[[:space:]]*#.*$//' -e 's/[[:space:]]*$//'
}

# The `cairn:` type of file $1, or empty.
fm_type_of() { fm_block "$1" | sed -n 's/^cairn:[[:space:]]*//p' | head -n1; }

# Scalar key $2 from cairn.toml $1, default $3. Subset TOML: key = "v" or bare.
toml_get() {
  local file="$1" key="$2" def="$3" val
  [ -f "$file" ] || { printf '%s\n' "$def"; return; }
  val=$(sed -n "s/^[[:space:]]*$key[[:space:]]*=[[:space:]]*//p" "$file" | head -n1)
  [ -z "$val" ] && { printf '%s\n' "$def"; return; }
  val=${val%%#*}
  val=$(printf '%s' "$val" | sed -e 's/[[:space:]]*$//' -e 's/^"//' -e 's/"$//')
  printf '%s\n' "$val"
}

is_kebab() { printf '%s' "$1" | grep -qE '^[a-z0-9][a-z0-9-]*$'; }

# List change directories under $1, excluding archive/, plus archive/* entries.
change_dirs() {
  local changes="$1" d b
  [ -d "$changes" ] || return 0
  for d in "$changes"/*/; do
    [ -d "$d" ] || continue
    b=$(basename "$d")
    [ "$b" = "archive" ] && continue
    printf '%s\n' "${d%/}"
  done
  if [ -d "$changes/archive" ]; then
    for d in "$changes/archive"/*/; do
      [ -d "$d" ] || continue
      printf '%s\n' "${d%/}"
    done
  fi
}

set -u

start=""
block=0
while [ $# -gt 0 ]; do
  case "$1" in
    --block) block=1; shift ;;
    -h|--help) printf 'usage: verify [dir] [--block]\n'; exit 0 ;;
    -*) die "unknown option: $1" ;;
    *) start="$1"; shift ;;
  esac
done
start="${start:-$PWD}"

VIOLATIONS=0
report() { printf '  [%s] %s: %s\n' "$1" "$2" "$3" >&2; VIOLATIONS=$((VIOLATIONS + 1)); }

if ! root=$(find_root "$start"); then
  printf 'cairn: no root found (no cairn/ directory or cairn.toml above %s)\n' "$start" >&2
  report C1 "$start" "no discoverable Cairn root"
  [ "$block" -eq 1 ] && exit 2 || exit 1
fi

toml="$root/cairn.toml"
base=$(toml_get "$toml" base "cairn")
specd=$(toml_get "$toml" spec "spec")
changesd=$(toml_get "$toml" changes "changes")
logd=$(toml_get "$toml" log "log")
require_delta=$(toml_get "$toml" require_delta "false")

BASE="$root/$base"
SPEC="$BASE/$specd"
CHANGES="$BASE/$changesd"
LOG="$BASE/$logd"

# C2: required directories
[ -d "$SPEC" ]    || report C2 "$SPEC"    "missing spec/ directory"
[ -d "$CHANGES" ] || report C2 "$CHANGES" "missing changes/ directory"
[ -d "$LOG" ]     || report C2 "$LOG"     "missing log/ directory"

# C3: every Cairn markdown file carries a valid cairn: type
while IFS= read -r f; do
  t=$(fm_type_of "$f")
  case "$t" in
    spec|change|tasks|delta|log) ;;
    "") report C3 "${f#$root/}" "missing cairn: frontmatter" ;;
    *)  report C3 "${f#$root/}" "unknown cairn type '$t'" ;;
  esac
done < <(find "$BASE" -type f -name '*.md' 2>/dev/null)

# C5: capability filenames are kebab-case
if [ -d "$SPEC" ]; then
  while IFS= read -r f; do
    cap=$(basename "$f" .md)
    is_kebab "$cap" || report C5 "${f#$root/}" "capability '$cap' is not kebab-case"
  done < <(find "$SPEC" -maxdepth 1 -type f -name '*.md' 2>/dev/null)
fi

# C4, C5, C6, C7, C9: per change
while IFS= read -r c; do
  [ -n "$c" ] || continue
  id=$(basename "$c")
  proposal="$c/proposal.md"
  tasks="$c/tasks.md"
  delta="$c/delta.md"

  is_kebab "$id" || report C5 "${c#$root/}" "change id '$id' is not kebab-case"

  if [ -f "$proposal" ]; then
    [ -n "$(fm_get "$proposal" id)" ]      || report C4 "${proposal#$root/}" "frontmatter missing 'id'"
    [ -n "$(fm_get "$proposal" status)" ]  || report C4 "${proposal#$root/}" "frontmatter missing 'status'"
    [ -n "$(fm_get "$proposal" created)" ] || report C4 "${proposal#$root/}" "frontmatter missing 'created'"
  else
    report C4 "${c#$root/}" "missing proposal.md"
  fi
  [ -f "$tasks" ] || report C4 "${c#$root/}" "missing tasks.md"

  status=$(fm_get "$proposal" status)

  # C6: delta headings are literal
  if [ -f "$delta" ]; then
    bad=$(grep -E '^## ' "$delta" | grep -vE '^## (ADDED|MODIFIED|REMOVED) Requirements$' || true)
    [ -n "$bad" ] && report C6 "${delta#$root/}" "non-conforming level-2 heading(s)"
  fi

  # C7: landed change has a matching log entry
  if [ "$status" = "landed" ]; then
    ls "$LOG"/*-"$id".md >/dev/null 2>&1 \
      || report C7 "${c#$root/}" "landed change has no log entry (log/YYYY-MM-DD-$id.md)"
  fi

  # C9: require_delta
  if [ "$require_delta" = "true" ] && [ "$status" != "abandoned" ] && [ ! -f "$delta" ]; then
    report C9 "${c#$root/}" "require_delta is set but delta.md is missing"
  fi
done < <(change_dirs "$CHANGES")

# C8: log filenames are dated
if [ -d "$LOG" ]; then
  while IFS= read -r f; do
    b=$(basename "$f")
    printf '%s' "$b" | grep -qE '^[0-9]{4}-[0-9]{2}-[0-9]{2}-[a-z0-9][a-z0-9-]*\.md$' \
      || report C8 "${f#$root/}" "log filename must be YYYY-MM-DD-<id>.md"
  done < <(find "$LOG" -maxdepth 1 -type f -name '*.md' 2>/dev/null)
fi

if [ "$VIOLATIONS" -gt 0 ]; then
  printf 'cairn: %d conformance violation(s) under %s\n' "$VIOLATIONS" "${BASE#$root/}" >&2
  [ "$block" -eq 1 ] && exit 2 || exit 1
fi
printf 'cairn: conformant (%s)\n' "$BASE"
