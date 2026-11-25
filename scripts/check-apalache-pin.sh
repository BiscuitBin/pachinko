#!/bin/sh
# Gate 2's check of tools/quint/apalache.txt, the Apalache pin (decision 0021).
# Offline and read-only. It reads only the pin file, tools/quint/package.json
# and tools/quint/package-lock.json, as they are in the checkout, never
# tools/quint/node_modules/, so it fires in CI, where that is absent. It fails
# unless the pin file holds exactly the keys quint=, apalache= and sha256=, once
# each, beside blank and `#` lines; apalache= and quint= are x.y.z and sha256=
# is 64 lowercase hex digits; and quint= equals both the @informalsystems/quint
# version tools/quint/package.json pins and the one
# tools/quint/package-lock.json locks. So every move of the Quint CLI's pin
# fails here until quint= is edited. What it proves is that quint= was edited,
# not that anyone read the new CLI's DEFAULT_APALACHE_VERSION_TAG:
# scripts/install-apalache.sh checks the tag. The file is read with sed and
# grep, never sourced; JSON is read with pixi's python, first on the Justfile's
# PATH.
set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$project_root"

LC_ALL=C
export LC_ALL

pin=tools/quint/apalache.txt
howto='docs/how-to/maintain-dependencies.md, "The Quint CLI"'

fail() {
    printf 'check-apalache-pin: %s\n' "$*" >&2
    exit 1
}

[ -f "$pin" ] || fail "$pin is missing"
[ -r "$pin" ] || fail "$pin is unreadable"

# Any line that is not blank, a comment or one of the three keys. grep exits 1
# when it selects nothing, which is the passing case; any other status is an
# error and fails.
other=$(grep -Evx -e '#.*' -e '' -e '(quint|apalache|sha256)=.*' "$pin") && status=0 || status=$?
case $status in
    0) fail "$pin has a line that is not quint=, apalache=, sha256=, a comment or blank: $other" ;;
    1) ;;
    *) fail "cannot read $pin" ;;
esac

for key in quint apalache sha256; do
    # grep -c prints 0 and exits 1 when nothing matches; an empty count fails below.
    count=$(grep -c "^$key=" "$pin") || :
    [ "$count" = 1 ] || fail "$pin must have exactly one $key= line; it has ${count:-none}"
done

quint=$(sed -n 's/^quint=//p' "$pin")
apalache=$(sed -n 's/^apalache=//p' "$pin")
sha256=$(sed -n 's/^sha256=//p' "$pin")

printf '%s\n' "$quint" | grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+' || fail "quint=$quint in $pin is not x.y.z"
printf '%s\n' "$apalache" | grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+' || fail "apalache=$apalache in $pin is not x.y.z"
printf '%s\n' "$sha256" | grep -Eqx '[0-9a-f]{64}' || fail "sha256= in $pin is not 64 lowercase hex digits: $sha256"

manifest=$(python -I -B -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["dependencies"]["@informalsystems/quint"])' tools/quint/package.json) ||
    fail "cannot read the @informalsystems/quint pin from tools/quint/package.json"
locked=$(python -I -B -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["packages"]["node_modules/@informalsystems/quint"]["version"])' tools/quint/package-lock.json) ||
    fail "cannot read the @informalsystems/quint version from tools/quint/package-lock.json"

if [ "$manifest" != "$quint" ] || [ "$locked" != "$quint" ]; then
    fail "$pin says quint=$quint, but tools/quint/package.json pins $manifest and tools/quint/package-lock.json locks $locked. After a Quint bump, read the new CLI's DEFAULT_APALACHE_VERSION_TAG, move apalache= and sha256= if it moved, then set quint= ($howto)"
fi
