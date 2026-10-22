#!/bin/sh
# Gate 8: libpachinko performs no effect itself (decision 0008). This is T00's
# version: the crate bans only, over libpachinko's normal dependency tree, which
# covers transitives and leaves development dependencies exempt. T10 adds the
# std-API bans and the fixture probe.
set -eu

banned='rand rand_core getrandom tokio async-std smol mio socket2'

# Captured, not piped: under `set -eu` a cargo failure aborts the script here
# instead of leaving an empty pipe to pass the loop below.
out=$(cargo tree -p libpachinko -e normal --prefix none --locked --offline)

# Positive control: a tree that does not name libpachinko itself proves nothing.
if ! printf '%s\n' "$out" | grep -q '^libpachinko '; then
    printf '%s\n' "$out" >&2
    printf '%s\n' 'effects: cargo tree did not list libpachinko itself' >&2
    exit 1
fi

# Each line reads `<name> v<version> [(<source>)]`; the name is matched as a
# whole field, so rand never matches rand_core and the reverse.
hits=$(printf '%s\n' "$out" | awk -v banned="$banned" '
    BEGIN { n = split(banned, list, " "); for (i = 1; i <= n; i++) ban[list[i]] = 1 }
    $1 in ban { print }
')
if [ -n "$hits" ]; then
    printf 'effects: libpachinko depends on a banned crate:\n%s\n' "$hits" >&2
    exit 1
fi

printf '%s\n' "effects: libpachinko's normal dependency tree names no banned crate."
printf '%s\n' 'effects: the std-API bans and the fixture probe arrive with T10.'
