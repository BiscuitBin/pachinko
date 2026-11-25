#!/bin/sh
# Apalache from the release archive tools/quint/apalache.txt pins by SHA-256,
# unpacked to .quint/apalache-dist-<v>/apalache, the path Quint 0.34.0 looks in
# before it downloads one itself (decision 0021). Two modes:
#
# - No argument (`just install-apalache`): install. It fails unless the pin file
#   passes scripts/check-apalache-pin.sh, the installed Quint CLI is the quint=
#   pin and its DEFAULT_APALACHE_VERSION_TAG is the apalache= pin. It skips, with
#   no network, only when the tree carries a stamp whose archive= is the pinned
#   hash and whose tree= is the digest of the tree as it is now, and the launcher
#   is executable. Any other tree directory, Quint's own download included, is
#   replaced (a symlink or non-directory is refused; see below): the archive
#   is downloaded into a fresh .quint/install-apalache.* directory, its hash
#   checked, unpacked, checked against its own listing and
#   stamped there, and only then moved into place. An interrupted run leaves the
#   old tree or none, never an unstamped tree that looks finished.
# - `--check -- <quint arguments>` (the `quint` recipe's guard): offline and
#   read-only. It refuses any option whose name contains `apalache`, except
#   --apalache-config, since such an option could make Quint download another
#   Apalache unchecked. Then, unless there are no arguments, the first is parse,
#   typecheck, run, test, repl or docs, or the only one is --help or --version,
#   it runs the install mode's checks and skip test, and refuses unless that
#   test would skip and pixi's java is executable.
#
# Both modes refuse a QUINT_HOME other than `.quint`, the value the Justfile
# exports, and a symlink or non-directory at .quint, the dist directory or the
# tree, so the guard, the installer and Quint name one directory. The tree
# digest is the SHA-256 of the sorted `<sha256>  <path>` lines of every regular
# file under apalache/ except the stamp; anything else in the tree (a symlink, a
# device) fails it. It covers content, not modes, so the launcher's mode is
# tested apart. The only path ever removed is the temporary directory.
set -eu

start=$(pwd -P)
project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$project_root"

LC_ALL=C
export LC_ALL

pin=tools/quint/apalache.txt
cli=tools/quint/node_modules/@informalsystems/quint
java=.pixi/envs/default/lib/jvm/bin/java
nl='
'

mode=install
if [ $# -gt 0 ]; then
    if [ "$1" = --check ] && [ "${2-}" = -- ]; then
        mode=check
        shift 2
    else
        printf '%s\n' 'usage: install-apalache.sh [--check -- <quint arguments>]' >&2
        exit 2
    fi
fi

fail() {
    if [ "$mode" = check ]; then
        printf 'install-apalache --check: refused: %s\n' "$*" >&2
    else
        printf 'install-apalache: %s\n' "$*" >&2
    fi
    exit 1
}

say() {
    printf 'install-apalache: %s\n' "$*"
}

# Quint resolves QUINT_HOME against its working directory, which is the
# recipe's; only the exact value the Justfile exports is accepted, so no path is
# ever resolved and all three name the same directory.
home() {
    [ "$start" = "$project_root" ] ||
        fail "run from the repository root, $project_root, as the Justfile does, not $start"
    [ "${QUINT_HOME-}" = .quint ] ||
        fail "QUINT_HOME must be .quint, the value the Justfile exports; other caches are not supported (it is '${QUINT_HOME-}')"
}

read_pin() {
    sh scripts/check-apalache-pin.sh || fail "$pin fails its check (above)"
    quint=$(sed -n 's/^quint=//p' "$pin")
    apalache=$(sed -n 's/^apalache=//p' "$pin")
    sha=$(sed -n 's/^sha256=//p' "$pin")
}

check_cli() {
    # The `quint` recipe runs this file; node resolves a symlinked entry point,
    # or a linked package directory, to its target, which could be another CLI
    # than the one read below.
    for link in tools/quint/node_modules tools/quint/node_modules/@informalsystems "$cli" "$cli/dist" "$cli/dist/src" "$cli/dist/src/cli.js"; do
        [ ! -L "$link" ] ||
            fail "$link is a symlink, so the quint recipe could run another CLI than the one checked; run just install-quint"
    done
    [ -f "$cli/dist/src/cli.js" ] ||
        fail "$cli/dist/src/cli.js, the entry point the quint recipe runs, is missing; run just install-quint"
    # The launcher is bash, which imports exported functions: one named java,
    # or any command it runs, would replace that command.
    if env | grep -q '^BASH_FUNC_'; then
        fail "the environment exports a bash function (BASH_FUNC_...), which Apalache's bash launcher would import; unset it before just quint"
    fi
    installed=$(python -I -B -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["version"])' "$cli/package.json") ||
        fail "cannot read the installed Quint CLI's version from $cli/package.json; run just install-quint"
    [ "$installed" = "$quint" ] ||
        fail "the installed Quint CLI is $installed, not quint=$quint from $pin; run just install-quint"
    tag=$(sed -n "s/^exports\.DEFAULT_APALACHE_VERSION_TAG = '\(.*\)';\$/\1/p" "$cli/dist/src/apalache.js") ||
        fail "cannot read DEFAULT_APALACHE_VERSION_TAG from $cli/dist/src/apalache.js"
    # sed matches nothing and still exits 0 when a later CLI writes the line otherwise.
    [ -n "$tag" ] ||
        fail "cannot read DEFAULT_APALACHE_VERSION_TAG from $cli/dist/src/apalache.js; read it by hand (docs/how-to/maintain-dependencies.md, \"The Quint CLI\")"
    [ "$tag" = "$apalache" ] ||
        fail "the installed Quint CLI $installed runs Apalache '$tag' (DEFAULT_APALACHE_VERSION_TAG), but $pin pins apalache=$apalache; see docs/how-to/maintain-dependencies.md, \"The Quint CLI\""
}

paths() {
    qh=.quint
    dist=$qh/apalache-dist-$apalache
    tree=$dist/apalache
    for p in "$qh" "$dist" "$tree"; do
        if [ -L "$p" ]; then
            fail "$p is a symlink; it must be a directory inside this worktree. Remove the symlink itself (not its target), then run just install-apalache"
        fi
        if [ -e "$p" ] && [ ! -d "$p" ]; then
            fail "$p is not a directory. Remove it, then run just install-apalache"
        fi
    done
}

# Sets h to the SHA-256 of the file $1, in lowercase hex.
sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        h=$(sha256sum -- "$1") || return 1
    else
        h=$(shasum -a 256 -- "$1") || return 1
    fi
    h=${h%% *}
    printf '%s\n' "$h" | grep -Eqx '[0-9a-f]{64}' || return 1
}

# One process, so a failed walk or read cannot yield a digest; it exits non-zero
# on any error and on any entry that is not a regular file or a directory.
digest_program='
import hashlib, os, stat, sys

def fail(message):
    sys.stderr.write("tree digest: " + message + "\n")
    sys.exit(1)

def raise_error(error):
    raise error

root = sys.argv[1]
lines = []
try:
    if not stat.S_ISDIR(os.lstat(root).st_mode):
        fail(root + " is not a directory")
    for directory, dirs, files in os.walk(root, onerror=raise_error):
        for name in dirs + files:
            path = os.path.join(directory, name)
            rel = os.path.relpath(path, root).replace(os.sep, "/")
            if "\n" in rel or "\\" in rel:
                fail("unsupported path " + repr(rel))
            mode = os.lstat(path).st_mode
            if stat.S_ISDIR(mode):
                continue
            if not stat.S_ISREG(mode):
                fail("not a regular file or directory: " + rel)
            if rel == ".pachinko-stamp":
                continue
            digest = hashlib.sha256()
            with open(path, "rb") as handle:
                for chunk in iter(lambda: handle.read(1 << 20), b""):
                    digest.update(chunk)
            lines.append((digest.hexdigest() + "  " + rel).encode("utf-8", "surrogateescape"))
except OSError as error:
    fail(str(error))
lines.sort()
print(hashlib.sha256(b"".join(line + b"\n" for line in lines)).hexdigest())
'

# Sets d to the digest of the tree $1.
tree_digest() {
    d=$(python -I -B -c "$digest_program" "$1") || return 1
    printf '%s\n' "$d" | grep -Eqx '[0-9a-f]{64}' || return 1
}

# The skip test. Returns 1, with the reason in `reason`, unless the tree is the
# one the stamp records, from the pinned archive, with an executable launcher.
# Called only as a condition, where set -e is off, so every test returns.
stamped_ok() {
    stamp=$tree/.pachinko-stamp
    launcher=$tree/bin/apalache-mc
    if [ ! -e "$tree" ]; then
        reason="no stamp: there is no tree at $tree"
        return 1
    fi
    if [ -L "$stamp" ] || [ ! -f "$stamp" ]; then
        reason="no stamp at $stamp"
        return 1
    fi
    if ! s=$(cat -- "$stamp" 2>/dev/null); then
        reason="stamp unreadable: $stamp"
        return 1
    fi
    case $s in
        "archive=$sha$nl"*) ;;
        *)
            reason="stamp archive mismatch: $stamp does not record the pinned sha256 $sha"
            return 1
            ;;
    esac
    if ! tree_digest "$tree"; then
        reason="walk failed: the tree under $tree cannot be digested"
        return 1
    fi
    if [ "$s" != "archive=$sha${nl}tree=$d" ]; then
        reason="tree digest mismatch: the files under $tree are not the ones $stamp records"
        return 1
    fi
    if [ -L "$launcher" ] || [ ! -f "$launcher" ] || [ ! -x "$launcher" ]; then
        reason="launcher not executable: $launcher"
        return 1
    fi
    return 0
}

java_ok() {
    [ -f "$java" ] && [ -x "$java" ] ||
        fail "pixi's java, $java, is missing or not executable; the launcher would run another java on PATH; run just sync"
}

# With `names`, refuses a listing on stdin with a member outside apalache/ or
# with an empty, `.` or `..` component, before anything is unpacked. With
# `tree`, also compares what tar unpacked under $1/apalache with that listing.
extract_program='
import os, stat, sys

def fail(message):
    sys.stderr.write("archive check: " + message + "\n")
    sys.exit(1)

def raise_error(error):
    raise error

root = sys.argv[1]
listed = set()
for member in sys.stdin.read().split("\n"):
    if member == "":
        continue
    parts = member.rstrip("/").split("/")
    if parts[0] != "apalache" or any(part in ("", ".", "..") for part in parts):
        fail("unexpected member " + repr(member))
    for depth in range(1, len(parts) + 1):
        listed.add("/".join(parts[:depth]))
if not listed:
    fail("the archive is empty")
if sys.argv[2] == "names":
    sys.exit(0)
found = set()
try:
    top = os.path.join(root, "apalache")
    if not stat.S_ISDIR(os.lstat(top).st_mode):
        fail("apalache/ is not a directory")
    found.add("apalache")
    for directory, dirs, files in os.walk(top, onerror=raise_error):
        for name in dirs + files:
            found.add(os.path.relpath(os.path.join(directory, name), root).replace(os.sep, "/"))
except OSError as error:
    fail(str(error))
for path in sorted(listed - found):
    fail("listed but not unpacked: " + path)
for path in sorted(found - listed):
    fail("unpacked but not listed: " + path)
'

check_mode() {
    home
    for arg in "$@"; do
        case $arg in
            -*)
                case ${arg%%=*} in
                    *[Aa][Pp][Aa][Ll][Aa][Cc][Hh][Ee]*)
                        case $arg in
                            --apalache-config | --apalache-config=*) ;;
                            *) fail "the option '$arg' could make Quint use or download another Apalache; the version is pinned in $pin and installed by just install-apalache" ;;
                        esac
                        ;;
                esac
                ;;
        esac
    done
    [ $# -gt 0 ] || exit 0
    case $1 in
        parse | typecheck | run | test | repl | docs) exit 0 ;;
    esac
    if [ $# -eq 1 ]; then
        case $1 in
            --help | --version) exit 0 ;;
        esac
    fi
    why="the first argument '$1' is not one of parse, typecheck, run, test, repl or docs, so the command may run Apalache"
    read_pin
    check_cli
    paths
    stamped_ok ||
        fail "Apalache $apalache is not installed as $pin pins it ($reason); run just install-apalache. Checked because $why"
    java_ok
    exit 0
}

install_mode() {
    home
    read_pin
    check_cli
    paths
    if stamped_ok; then
        say "Apalache $apalache is installed and matches its stamp; nothing to do."
        exit 0
    fi
    say "installing Apalache $apalache into $tree: $reason"

    mkdir -p "$qh"
    paths
    # The traps go first, so a signal right after mktemp still removes its directory.
    tmp=''
    trap '[ -z "$tmp" ] || rm -rf -- "${tmp:?}"' EXIT
    trap 'exit 1' HUP INT TERM
    tmp=$(mktemp -d "$qh/install-apalache.XXXXXX")

    url=https://github.com/apalache-mc/apalache/releases/download/v$apalache/apalache.tgz
    say "downloading $url"
    curl -fsSL --proto '=https' --proto-redir '=https' --tlsv1.2 -o "$tmp/apalache.tgz" "$url" ||
        fail "the download of $url failed"
    sha256_of "$tmp/apalache.tgz" || fail "cannot hash the downloaded archive"
    [ "$h" = "$sha" ] ||
        fail "checksum mismatch for $url: $pin pins sha256 $sha, the download has $h; nothing was installed"

    # GNU tar reads TAR_OPTIONS, BSD tar TAR_READER_OPTIONS and TAR_WRITER_OPTIONS,
    # and both TAPE; any of them could change what is listed or unpacked.
    list=$(env -u TAR_OPTIONS -u TAR_READER_OPTIONS -u TAR_WRITER_OPTIONS -u TAPE tar -tzf "$tmp/apalache.tgz") ||
        fail "cannot list the archive"
    [ -n "$list" ] || fail "the archive is empty"
    printf '%s\n' "$list" | python -I -B -c "$extract_program" "$tmp" names ||
        fail "the archive holds a member outside apalache/"
    env -u TAR_OPTIONS -u TAR_READER_OPTIONS -u TAR_WRITER_OPTIONS -u TAPE tar -xzf "$tmp/apalache.tgz" -C "$tmp" ||
        fail "cannot unpack the archive"
    printf '%s\n' "$list" | python -I -B -c "$extract_program" "$tmp" tree ||
        fail "the unpacked tree does not match the archive's listing"
    [ -f "$tmp/apalache/lib/apalache.jar" ] && [ ! -L "$tmp/apalache/lib/apalache.jar" ] ||
        fail "the archive holds no apalache/lib/apalache.jar"
    [ -f "$tmp/apalache/bin/apalache-mc" ] && [ ! -L "$tmp/apalache/bin/apalache-mc" ] && [ -x "$tmp/apalache/bin/apalache-mc" ] ||
        fail "the archive holds no executable apalache/bin/apalache-mc"
    tree_digest "$tmp/apalache" || fail "cannot digest the unpacked tree"
    printf 'archive=%s\ntree=%s\n' "$sha" "$d" >|"$tmp/apalache/.pachinko-stamp"

    mkdir -p "$dist"
    paths
    if [ -e "$tree" ] || [ -L "$tree" ]; then
        mv -- "$tree" "$tmp/old"
    fi
    if [ -e "$tree" ] || [ -L "$tree" ]; then
        fail "$tree reappeared while installing, so another process is writing it; run just install-apalache again once it has finished"
    fi
    mv -- "$tmp/apalache" "$tree"
    stamped_ok ||
        fail "the installed tree fails its own check ($reason); another process may have written it; run just install-apalache again"
    say "installed Apalache $apalache (sha256 $sha) into $tree."
}

if [ "$mode" = check ]; then
    check_mode "$@"
else
    install_mode
fi
