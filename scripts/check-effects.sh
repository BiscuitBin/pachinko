#!/bin/sh
# Gate 8: libpachinko performs no effect itself (decision 0008). Two mechanisms,
# each checked three ways (CONVENTIONS.md §9):
#
# - Crates: libpachinko's normal dependency tree, from `cargo tree`, names no
#   banned crate. It covers transitives and leaves development dependencies
#   exempt, so a test may use a seeded generator.
# - std APIs: clippy's disallowed-methods and disallowed-types, from the entries
#   in crates/libpachinko/clippy.toml whose reason begins `effect:`. Clippy
#   resolves names, so an alias, a glob, a macro body or a method on a value
#   passed in is caught as surely as a full path. Clippy reads that file for
#   libpachinko alone, so pachinko keeps every effect.
#
# The checks, in order: (0) the libpachinko clippy.toml still restates the root
# one, which it shadows; (1) libpachinko trips no ban; (2) the fixture under
# tests/fixtures/effects-violation trips every ban exactly once, through the same
# commands and matching, so a ban that cannot fire fails the gate; (3) pachinko
# trips no std-API ban. Offline throughout: the fixture's crates are path stubs.
set -eu

# The crate bans: one name per line, then why. Only the first field is read.
banned_crates='
rand       randomness, reached through a trait libpachinko defines
rand_core  randomness, the same
getrandom  randomness from the operating system
tokio      an async runtime: threads, the clock and the network
async-std  an async runtime, the same
smol       an async runtime, the same
mio        an operating-system event loop over the network
socket2    the network
'
# The std-API bans live in clippy's file, which is where clippy reads them.
api_bans=crates/libpachinko/clippy.toml
fixture=tests/fixtures/effects-violation
# Its own target directory, so this gate's clippy runs never invalidate gate 6's.
target=target/effects

# A CLIPPY_CONF_DIR in the caller's environment would replace the file clippy
# finds for each crate; every run below sets it, or leaves it unset, on purpose.
unset CLIPPY_CONF_DIR

crate_names=$(printf '%s\n' "$banned_crates" | awk 'NF { print $1 }' | sort)
api_paths=$(sed -n 's/^ *{ path = "\([^"]*\)", reason = "effect:.*/\1/p' "$api_bans" | sort)
if [ -z "$crate_names" ] || [ -z "$api_paths" ]; then
    printf '%s\n' "effects: no crate bans or no std-API bans were read from this script and $api_bans" >&2
    exit 1
fi

# Joins lines with spaces, the form `awk -v` takes portably. No name or path
# here holds a space.
words() {
    printf '%s\n' "$1" | tr '\n' ' '
}

# Prints the lines of the first list that are not in the second.
minus() {
    printf '%s\n' "$1" | awk -v drop="$(words "$2")" '
        BEGIN { n = split(drop, list, " "); for (i = 1; i <= n; i++) skip[list[i]] = 1 }
        NF && !($0 in skip) { print }
    '
}

# Prints the normal dependency tree; the arguments say whose. Captured by the
# caller, so a cargo failure aborts under `set -e` instead of passing as an
# empty tree.
crate_tree() {
    cargo tree "$@" -e normal --prefix none --all-features --locked --offline
}

# Prints the banned names in a tree, one per occurrence. Each line reads
# `<name> v<version> [(<source>)]`; the name is matched as a whole field, so
# rand never matches rand_core and the reverse.
banned_in_tree() {
    printf '%s\n' "$1" | awk -v banned="$(words "$crate_names")" '
        BEGIN { n = split(banned, list, " "); for (i = 1; i <= n; i++) ban[list[i]] = 1 }
        $1 in ban { print $1 }
    ' | sort
}

# Prints the effect paths clippy reported, one per finding. Other lints, the
# root file's HashMap ban included, are gate 6's business.
banned_in_clippy() {
    tick=$(printf '\140')
    printf '%s\n' "$1" | sed -n "s/^[a-z]*: use of a disallowed [a-z]* $tick\([^$tick]*\)$tick\$/\1/p" |
        awk -v paths="$(words "$api_paths")" '
            BEGIN { n = split(paths, list, " "); for (i = 1; i <= n; i++) ban[list[i]] = 1 }
            $0 in ban { print }
        ' | sort
}

# Runs clippy with the given arguments and all features, reading clippy.toml from
# the directory in $1, or from wherever clippy finds it when $1 is empty, and sets
# `clippy_out` and `clippy_status`. The directory goes through `env`, not a
# `VAR=value` prefix: POSIX leaves a prefix on a function call in force after it
# returns in some shells, and it would then leak into the next run. Warnings stay warnings, so the status is
# non-zero only when the code does not compile. Cargo replays a fresh crate's
# cached warnings, so a second run reports what the first did.
run_clippy() {
    conf=$1
    shift
    clippy_status=0
    if [ -n "$conf" ]; then
        clippy_out=$(env CLIPPY_CONF_DIR="$conf" cargo clippy "$@" --all-features --locked --offline \
            --target-dir "$target" 2>&1) || clippy_status=$?
    else
        clippy_out=$(cargo clippy "$@" --all-features --locked --offline --target-dir "$target" 2>&1) ||
            clippy_status=$?
    fi
}

# Check 0. The comparison drops comments, blank lines and the effect entries,
# which only the libpachinko file has, and its disallowed-methods line, a key
# the root file does not set.
normalise() {
    grep -v -e '^ *#' -e '^ *$' -e 'reason = "effect:' -e '^disallowed-methods = \[$' "$1" | sort -u
}
root_lines=$(normalise clippy.toml)
lib_lines=$(normalise "$api_bans")
if [ "$root_lines" != "$lib_lines" ]; then
    printf '%s\n' "effects: $api_bans differs from clippy.toml beyond its effect entries." >&2
    mkdir -p "$target"
    printf '%s\n' "$root_lines" > "$target/clippy-root.txt"
    printf '%s\n' "$lib_lines" > "$target/clippy-libpachinko.txt"
    diff "$target/clippy-root.txt" "$target/clippy-libpachinko.txt" >&2 || :
    printf '%s\n' "clippy reads only the crate's file for libpachinko; restate the root file there." >&2
    exit 1
fi

# Check 1, crates.
tree=$(crate_tree -p libpachinko)
# Positive control: a tree that does not name libpachinko itself proves nothing.
if ! printf '%s\n' "$tree" | grep -q '^libpachinko '; then
    printf '%s\n' "$tree" >&2
    printf '%s\n' 'effects: cargo tree did not list libpachinko itself' >&2
    exit 1
fi
hits=$(banned_in_tree "$tree")
if [ -n "$hits" ]; then
    printf '%s\n' "$tree" >&2
    printf 'effects: libpachinko depends on a banned crate:\n%s\nwant none\n' "$hits" >&2
    exit 1
fi

# Check 1, std APIs. libpachinko's own clippy.toml is named outright, which is
# the file clippy would find for it anyway.
run_clippy "$PWD/crates/libpachinko" -p libpachinko --all-targets
hits=$(banned_in_clippy "$clippy_out")
if [ -n "$hits" ] || [ "$clippy_status" -ne 0 ]; then
    printf '%s\n' "$clippy_out" >&2
    printf 'effects: clippy over libpachinko exited %s, want 0\nbanned APIs used:\n%s\nwant none\n' \
        "$clippy_status" "$hits" >&2
    exit 1
fi

# Check 2. The fixture is its own workspace, so the same commands reach it
# through --manifest-path. Clippy would find the root clippy.toml for it, so the
# libpachinko file is named outright.
tree=$(crate_tree --manifest-path "$fixture/Cargo.toml")
got=$(banned_in_tree "$tree")
if [ "$got" != "$crate_names" ]; then
    printf '%s\n' "$tree" >&2
    printf 'effects: the crate probe did not see each banned crate exactly once\ncrates banned:\n%s\ncrates seen:\n%s\n' \
        "$crate_names" "$got" >&2
    exit 1
fi
run_clippy "$PWD/crates/libpachinko" --manifest-path "$fixture/Cargo.toml"
got=$(banned_in_clippy "$clippy_out")
if [ "$clippy_status" -ne 0 ] || [ "$got" != "$api_paths" ]; then
    printf '%s\n' "$clippy_out" >&2
    printf 'effects: the API probe exited %s, want 0, and must fire every ban exactly once\n' "$clippy_status" >&2
    printf 'silent:\n%s\n' "$(minus "$api_paths" "$got")" >&2
    printf 'fired more than once:\n%s\n' "$(printf '%s\n' "$got" | uniq -d)" >&2
    exit 1
fi

# Check 3. pachinko is linted with the root clippy.toml, as gate 6 lints it, and
# may use every effect. The crate bans need no such check: `-p libpachinko`
# scopes them by construction.
run_clippy '' -p pachinko --all-targets
hits=$(banned_in_clippy "$clippy_out")
if [ -n "$hits" ] || [ "$clippy_status" -ne 0 ]; then
    printf '%s\n' "$clippy_out" >&2
    printf 'effects: clippy over pachinko exited %s, want 0\nstd-API bans fired:\n%s\nwant none: the bans are libpachinko'"'"'s alone\n' \
        "$clippy_status" "$hits" >&2
    exit 1
fi

printf 'effects: libpachinko names none of %s banned crates and uses none of %s banned std APIs;\n' \
    "$(printf '%s\n' "$crate_names" | wc -l | tr -d ' ')" "$(printf '%s\n' "$api_paths" | wc -l | tr -d ' ')"
printf '%s\n' 'effects: the fixture trips every ban exactly once, and pachinko trips none.'
