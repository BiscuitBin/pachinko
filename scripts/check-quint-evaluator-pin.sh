#!/bin/sh
# Gate 2's check of tools/quint/evaluator.txt, the Rust evaluator's pin
# (decision 0021). Offline and read-only. It reads only the pin file,
# tools/quint/package.json, tools/quint/package-lock.json and pyproject.toml,
# as they are in the checkout, never tools/quint/node_modules/ or .quint/, so it
# fires in CI, where neither exists. It fails unless the pin file is printable
# ASCII and newlines and holds exactly
# the keys quint= and evaluator=, once each, and one sha256.<platform>= line for
# each platform pyproject.toml lists under [tool.pixi.workspace] and for no
# other, beside blank and `#` lines; quint= is x.y.z, evaluator= is v followed
# by x.y.z, and each hash is 64 lowercase hex digits; and quint= equals both the
# @informalsystems/quint version tools/quint/package.json pins and the one
# tools/quint/package-lock.json locks. So every move of the Quint CLI's pin
# fails here until quint= is edited, and a platform added to pixi fails here
# until its hash is recorded. What it proves is that quint= was edited, not that
# anyone read the new CLI's QUINT_EVALUATOR_VERSION:
# scripts/install-quint-evaluator.sh checks the constant. The file is read with
# sed and grep, never sourced; JSON and TOML are read with pixi's python, first
# on the Justfile's PATH.
set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$project_root"

LC_ALL=C
export LC_ALL

pin=tools/quint/evaluator.txt
howto='docs/how-to/maintain-dependencies.md, "Moving the evaluator pin"'

fail() {
    printf 'check-quint-evaluator-pin: %s\n' "$*" >&2
    exit 1
}

[ -f "$pin" ] || fail "$pin is missing"
[ -r "$pin" ] || fail "$pin is unreadable"

# Printable ASCII and newlines only, checked on the bytes: dash drops a NUL
# from what sed prints, so a value read below could pass its format check with
# a byte the file holds after it. Exit 3 is a bad byte; any other failure is a
# read error.
bad=$(python -I -B -c '
import sys
with open(sys.argv[1], "rb") as handle:
    data = handle.read()
for offset, byte in enumerate(data):
    if byte != 10 and not 32 <= byte <= 126:
        print("byte 0x%02x at offset %d" % (byte, offset))
        sys.exit(3)
' "$pin") && status=0 || status=$?
case $status in
    0) ;;
    3) fail "$pin holds a byte other than printable ASCII or a newline ($bad); write it as plain ASCII" ;;
    *) fail "cannot read $pin" ;;
esac

# Any line that is not blank, a comment or one of the keys. grep exits 1 when it
# selects nothing, which is the passing case; any other status is an error.
other=$(grep -Evx -e '#.*' -e '' -e '(quint|evaluator)=.*' -e 'sha256\.[A-Za-z0-9_-]+=.*' "$pin") && status=0 || status=$?
case $status in
    0) fail "$pin has a line that is not quint=, evaluator=, sha256.<platform>=, a comment or blank: $other" ;;
    1) ;;
    *) fail "cannot read $pin" ;;
esac

for key in quint evaluator; do
    # grep -c prints 0 and exits 1 when nothing matches; an empty count fails below.
    count=$(grep -c "^$key=" "$pin") || :
    [ "$count" = 1 ] || fail "$pin must have exactly one $key= line; it has ${count:-none}"
done

quint=$(sed -n 's/^quint=//p' "$pin")
evaluator=$(sed -n 's/^evaluator=//p' "$pin")

printf '%s\n' "$quint" | grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+' || fail "quint=$quint in $pin is not x.y.z"
printf '%s\n' "$evaluator" | grep -Eqx 'v[0-9]+\.[0-9]+\.[0-9]+' ||
    fail "evaluator=$evaluator in $pin is not v followed by x.y.z, as the CLI's QUINT_EVALUATOR_VERSION spells it"

# The platforms pixi solves for, one per line, from one process that fails on a
# missing or malformed list.
platforms=$(python -I -B -c '
import sys, tomllib
with open(sys.argv[1], "rb") as handle:
    data = tomllib.load(handle)
platforms = data.get("tool", {}).get("pixi", {}).get("workspace", {}).get("platforms")
if not isinstance(platforms, list) or not platforms:
    sys.exit("no platforms list under [tool.pixi.workspace]")
seen = set()
for p in platforms:
    if not isinstance(p, str) or not p or not all(c.isalnum() or c in "_-" for c in p) or not p.isascii():
        sys.exit("unsupported platform name " + repr(p))
    if p in seen:
        sys.exit("duplicate platform " + p)
    seen.add(p)
    print(p)
' pyproject.toml) || fail "cannot read the platforms under [tool.pixi.workspace] in pyproject.toml"
[ -n "$platforms" ] || fail "pyproject.toml lists no platforms"

for p in $platforms; do
    count=$(grep -c "^sha256\.$p=" "$pin") || :
    [ "$count" = 1 ] || fail "$pin must have exactly one sha256.$p= line, for the pixi platform $p; it has ${count:-none}"
    sha=$(sed -n "s/^sha256\.$p=//p" "$pin")
    printf '%s\n' "$sha" | grep -Eqx '[0-9a-f]{64}' || fail "sha256.$p= in $pin is not 64 lowercase hex digits: $sha"
done

# A hash for a platform pixi does not list. One process, whose exit status is
# checked, so a failed read is a failure and never an empty list.
# It prints the first such key, or nothing.
extra=$(python -I -B -c '
import re, sys
platforms = set(sys.argv[2].split())
with open(sys.argv[1], encoding="utf-8") as handle:
    for line in handle:
        match = re.match(r"sha256\.([A-Za-z0-9_-]+)=", line)
        if match and match.group(1) not in platforms:
            print(match.group(1))
            break
' "$pin" "$platforms") || fail "cannot read the sha256.<platform>= keys in $pin"
[ -z "$extra" ] ||
    fail "$pin has sha256.$extra=, but $extra is not a platform under [tool.pixi.workspace] in pyproject.toml"

# The npm pins are compared with quint= inside Python, as decoded strings, so a
# value the shell would trim (a trailing newline) is a mismatch, not a pass.
# Exit 3 is a mismatch; any other failure is a read error.
npm=$(python -I -B -c '
import json, sys
quint = sys.argv[1]
with open(sys.argv[2], encoding="utf-8") as handle:
    manifest = json.load(handle)["dependencies"]["@informalsystems/quint"]
with open(sys.argv[3], encoding="utf-8") as handle:
    locked = json.load(handle)["packages"]["node_modules/@informalsystems/quint"]["version"]
if manifest == quint and locked == quint:
    sys.exit(0)
print("tools/quint/package.json pins %r and tools/quint/package-lock.json locks %r" % (manifest, locked))
sys.exit(3)
' "$quint" tools/quint/package.json tools/quint/package-lock.json) && status=0 || status=$?
case $status in
    0) ;;
    3) fail "$pin says quint=$quint, but $npm. After a Quint bump, read the new CLI's QUINT_EVALUATOR_VERSION, move evaluator= and every sha256.<platform>= if it moved, then set quint= ($howto)" ;;
    *) fail "cannot read the @informalsystems/quint pin from tools/quint/package.json and tools/quint/package-lock.json" ;;
esac
