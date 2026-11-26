#!/bin/sh
PATH=/usr/bin:/bin:/usr/sbin:/sbin
export PATH
/usr/bin/env | /usr/bin/grep -q '^BASH_FUNC_'
case $? in
    1) ;;
    0)
        /usr/bin/printf '%s\n' 'check-smolvm-pin: refused: the environment exports a shell function (BASH_FUNC_...), which /bin/sh would run in place of a command of this script; unset it, then run again' >&2
        exit 1
        ;;
    *)
        /usr/bin/printf '%s\n' 'check-smolvm-pin: refused: cannot read the environment to check it for an exported shell function' >&2
        exit 1
        ;;
esac
# Gate 2's check of tools/smolvm/smolvm.txt, the smolvm pin (decision 0022).
# Offline and read-only; it reads only the pin file, as it is in the checkout, so
# it fires in CI, where nothing is installed. It fails, exit 1, unless the file
# holds exactly the keys version=, asset= and sha256=, once each, beside blank
# (empty or whitespace-only) and `#` lines; version= is x.y.z; asset= is the
# darwin-arm64 tarball of that version, smolvm-<version>-darwin-arm64.tar.gz; and
# sha256= is 64 lowercase hex digits. A symbolic link at tools, tools/smolvm or the
# pin file is refused, so the pin is read from the checkout itself. Each failure
# of the pin prints `tools/smolvm/smolvm.txt: <key>: <reason>`. The file is read
# with sed and grep, never sourced. PATH is fixed on the first line and an
# exported shell function refused on the next (as is an environment that cannot
# be read); the root is found with shell builtins and every helper is named by its
# absolute path, so none is looked up on the caller's PATH.
set -eu

case $0 in
    */*) here=${0%/*} ;;
    *) here=. ;;
esac
CDPATH='' cd -- "$here/.." || exit 1
LC_ALL=C
export LC_ALL

pin=tools/smolvm/smolvm.txt

fail() {
    /usr/bin/printf '%s: %s: %s\n' "$pin" "$1" "$2" >&2
    exit 1
}

for p in tools tools/smolvm "$pin"; do
    [ ! -L "$p" ] || fail file "$p is a symbolic link; the pin must be read from the checkout itself"
done
[ -f "$pin" ] || fail file "missing, or not a regular file"
[ -r "$pin" ] || fail file "unreadable"

# Any line that is not blank, a comment or one of the three keys. grep exits 1
# when it selects nothing, which is the passing case; any other status fails.
other=$(/usr/bin/grep -Evx -e '#.*' -e '[[:space:]]*' -e '(version|asset|sha256)=.*' "$pin") && status=0 || status=$?
case $status in
    0)
        line=$(/usr/bin/printf '%s\n' "$other" | /usr/bin/sed -n 1p)
        key=${line%%=*}
        [ "$key" != "$line" ] || key=line
        fail "$key" "not one of version=, asset= or sha256=, a comment or blank: $line"
        ;;
    1) ;;
    *) fail file "cannot be read" ;;
esac

for key in version asset sha256; do
    # grep -c prints 0 and exits 1 when nothing matches; an empty count fails below.
    count=$(/usr/bin/grep -c "^$key=" "$pin") || :
    [ "$count" = 1 ] || fail "$key" "must appear exactly once; it appears ${count:-0} times"
done

version=$(/usr/bin/sed -n 's/^version=//p' "$pin")
asset=$(/usr/bin/sed -n 's/^asset=//p' "$pin")
sha256=$(/usr/bin/sed -n 's/^sha256=//p' "$pin")

/usr/bin/printf '%s\n' "$version" | /usr/bin/grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+' ||
    fail version "'$version' is not x.y.z"
[ "$asset" = "smolvm-$version-darwin-arm64.tar.gz" ] ||
    fail asset "'$asset' is not smolvm-$version-darwin-arm64.tar.gz, the darwin-arm64 tarball of version=$version"
/usr/bin/printf '%s\n' "$sha256" | /usr/bin/grep -Eqx '[0-9a-f]{64}' ||
    fail sha256 "'$sha256' is not 64 lowercase hex digits"
