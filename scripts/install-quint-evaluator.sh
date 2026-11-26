#!/bin/sh
# The Rust evaluator from the release archive tools/quint/evaluator.txt pins by
# SHA-256 for this platform, unpacked to .quint/rust-evaluator-<v>/, the
# directory Quint 0.34.0 looks in before it downloads one itself, unchecked
# (decision 0021). Quint runs it for `run`, `test`, the REPL and a `verify` that
# reports a violation or an observation. Three modes:
#
# - No argument (`just install-quint-evaluator`): install. It fails unless the
#   pin file passes scripts/check-quint-evaluator-pin.sh, pixi's node names a
#   platform the pin lists, the installed Quint CLI is the quint= pin and its
#   QUINT_EVALUATOR_VERSION is the evaluator= pin. It skips, with no network,
#   only when the directory carries a stamp whose archive= is this platform's
#   pinned hash and whose tree= is the digest of the directory as it is now, and
#   quint_evaluator is an executable regular file. Any other directory there,
#   Quint's own download included, is replaced (a symlink or non-directory is
#   refused): the archive is downloaded into a fresh
#   .quint/install-quint-evaluator.* directory, its hash checked, its members
#   checked before anything is unpacked, unpacked, checked against its listing
#   and stamped there, and only then moved into place. An interrupted run leaves
#   the old directory or none, never an unstamped one that looks finished.
# - `--check -- <quint arguments>` (the `quint` recipe's guard): offline and
#   read-only. Unless the first argument is parse, typecheck, compile or docs,
#   or the only one is --help or --version, it runs the install mode's checks
#   and skip test, and refuses unless that test would skip. It reads only the
#   first argument, never --backend, so `run --backend typescript` is refused
#   without the install too.
# - `--check-archive <file>`: offline and read-only; the member check alone, on
#   the archive given, so that it can be tested without a download.
#
# Every mode but the last refuses a QUINT_HOME other than `.quint`, the value
# the Justfile exports, and a symlink or non-directory at .quint or the
# evaluator's directory, so the guard, the installer and Quint name one
# directory. The tree digest is the SHA-256 of the sorted `<sha256>  <path>`
# lines of every regular file under the directory except the stamp; anything
# else there (a symlink, a device) fails it. It covers content, not modes, so
# the executable's mode is tested apart. The only path ever removed is the
# temporary directory.
set -eu

start=$(pwd -P)
project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$project_root"

LC_ALL=C
export LC_ALL

pin=tools/quint/evaluator.txt
cli=tools/quint/node_modules/@informalsystems/quint
bm=$cli/dist/src/rust/binaryManager.js
api='https://api.github.com/repos/quint-co/quint/releases/tags/evaluator/'
howto='docs/how-to/maintain-dependencies.md, "Moving the evaluator pin"'
nl='
'

mode=install
if [ $# -gt 0 ]; then
    if [ "$1" = --check ] && [ "${2-}" = -- ]; then
        mode=check
        shift 2
    elif [ "$1" = --check-archive ] && [ $# -eq 2 ]; then
        mode=archive
        shift
    else
        printf '%s\n' 'usage: install-quint-evaluator.sh [--check -- <quint arguments> | --check-archive <file>]' >&2
        exit 2
    fi
fi

fail() {
    if [ "$mode" = check ]; then
        # Every refusal names the recipe that installs the evaluator, whichever
        # check refused.
        case "$*" in
            *install-quint-evaluator*) ;;
            *) set -- "$*; once that is fixed, run just install-quint-evaluator" ;;
        esac
        printf 'install-quint-evaluator --check: refused: %s\n' "$*" >&2
    else
        printf 'install-quint-evaluator: %s\n' "$*" >&2
    fi
    exit 1
}

say() {
    printf 'install-quint-evaluator: %s\n' "$*"
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
    sh scripts/check-quint-evaluator-pin.sh || fail "$pin fails its check (above)"
    quint=$(sed -n 's/^quint=//p' "$pin")
    evaluator=$(sed -n 's/^evaluator=//p' "$pin")
}

# Maps the machine to a pixi platform and the asset the CLI would fetch for it
# (binaryManager.js, inferAssetAndExecutableNames). The CLI picks its asset from Node's process.platform and process.arch, in the
# node its `#!/usr/bin/env node` line finds on the recipe's PATH, so the same
# node is asked here: pixi's, which must be the first node on PATH, run with
# NODE_OPTIONS unset as the `quint` recipe runs it. It runs with an empty
# environment, since variables such as NODE_V8_COVERAGE or NODE_COMPILE_CACHE
# make node write files, and the guard is read-only; the platform and the
# architecture do not depend on the environment.
platform() {
    node=$project_root/.pixi/envs/default/bin/node
    [ -f "$node" ] && [ -x "$node" ] || fail "pixi's node, $node, is missing or not executable; run just sync"
    first=$(command -v node) || fail "no node on PATH; run the recipe through just, which puts pixi's first"
    # The Justfile's PATH names the checkout as just was given it, which may
    # pass through a symlink, so the two are compared as files (device and
    # inode, `test -ef`), not as spellings: no path is resolved, and a missing
    # or unreadable file is a mismatch.
    [ "$first" -ef "$node" ] ||
        fail "the first node on PATH is $first, not pixi's $node, so the CLI could run on another platform than the one asked; run the recipe through just"
    # The recipe runs the CLI with pixi's Java bin directory put before this
    # PATH, so a node there would run the CLI in place of the one asked.
    jvm_node=$project_root/.pixi/envs/default/lib/jvm/bin/node
    [ ! -e "$jvm_node" ] && [ ! -L "$jvm_node" ] ||
        fail "$jvm_node exists, and the quint recipe puts that directory first on the CLI's PATH, so the CLI would run on it, not on pixi's $node; run just sync"
    machine=$(env -i "$node" -p 'process.platform + " " + process.arch') ||
        fail "pixi's node could not report its platform"
    case $machine in
        'linux x64')
            plat=linux-64
            asset=quint_evaluator-x86_64-unknown-linux-gnu.tar.gz
            ;;
        'darwin arm64')
            plat=osx-arm64
            asset=quint_evaluator-aarch64-apple-darwin.tar.gz
            ;;
        *)
            listed=$(sed -n 's/^sha256\.\([A-Za-z0-9_-]*\)=.*/\1/p' "$pin" | tr '\n' ' ')
            fail "pixi's node runs as '$machine' (process.platform, process.arch), which is none of the platforms $pin pins: ${listed% }"
            ;;
    esac
    sha=$(sed -n "s/^sha256\.$plat=//p" "$pin")
    [ -n "$sha" ] || fail "$pin has no sha256.$plat= line for this machine ('$machine')"
}

check_cli() {
    # The `quint` recipe runs cli.js, and node resolves a symlinked entry point,
    # or a linked package directory, to its target, which could be another CLI
    # than the one read below. config.js builds the directory the evaluator is
    # looked up in, so it is on the path too. The lines that build that
    # directory are checked below, so a release that changes them fails; a
    # change elsewhere in these files, or in another module of the CLI, is not
    # detected, even one that moves the lookup: the guard checks the installed
    # package's text, not what node loads.
    for link in tools tools/quint tools/quint/node_modules tools/quint/node_modules/@informalsystems "$cli" "$cli/package.json" "$cli/dist" "$cli/dist/src" "$cli/dist/src/cli.js" "$cli/dist/src/config.js" "$cli/dist/src/rust" "$bm"; do
        [ ! -L "$link" ] ||
            fail "$link is a symlink, so the quint recipe could run another CLI than the one checked; run just install-quint"
    done
    [ -f "$cli/dist/src/cli.js" ] ||
        fail "$cli/dist/src/cli.js, the entry point the quint recipe runs, is missing; run just install-quint"
    [ -f "$cli/dist/src/config.js" ] ||
        fail "$cli/dist/src/config.js, where the CLI builds the evaluator's directory, is missing; run just install-quint"
    [ -f "$bm" ] ||
        fail "$bm, where the CLI names its evaluator, is missing; run just install-quint"
    # The CLI loads these two with extensionless require() calls ("../config",
    # "./binaryManager"), and node tries the exact name before adding .js, so an
    # entry there, a symlink included, would be loaded in place of the file
    # checked above.
    for shadow in "$cli/dist/src/config" "${bm%.js}"; do
        [ ! -e "$shadow" ] && [ ! -L "$shadow" ] ||
            fail "$shadow exists beside $shadow.js, and node's require() would load it in place of $shadow.js; remove it, then run just install-quint"
    done
    installed=$(python -I -B -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["version"])' "$cli/package.json") ||
        fail "cannot read the installed Quint CLI's version from $cli/package.json; run just install-quint"
    [ "$installed" = "$quint" ] ||
        fail "the installed Quint CLI is $installed, not quint=$quint from $pin; run just install-quint"
    const=$(sed -n "s/^exports\.QUINT_EVALUATOR_VERSION = '\(.*\)';\$/\1/p" "$bm") ||
        fail "cannot read QUINT_EVALUATOR_VERSION from $bm"
    # sed matches nothing and still exits 0 when a later CLI writes the line otherwise.
    case $const in
        '' | *"$nl"*)
            fail "cannot read one QUINT_EVALUATOR_VERSION from $bm; read it by hand ($howto)"
            ;;
    esac
    # Every line that names the constant must be one of the three 0.34.0 has, so
    # a later assignment (bracket notation, a second export) or the assignment
    # above sitting in a comment fails rather than passing on the line sed
    # read. A write that never spells the name is not seen.
    uses=$(grep -F QUINT_EVALUATOR_VERSION "$bm") ||
        fail "cannot find QUINT_EVALUATOR_VERSION in $bm; read it by hand ($howto)"
    [ "$uses" = "exports.QUINT_EVALUATOR_VERSION = void 0;${nl}exports.QUINT_EVALUATOR_VERSION = '$const';${nl}async function getRustEvaluatorPath(version = exports.QUINT_EVALUATOR_VERSION) {" ] ||
        fail "$bm names QUINT_EVALUATOR_VERSION on lines other than the three this check knows, so the value the CLI uses may not be '$const'; read it by hand ($howto)"
    [ "$const" = "$evaluator" ] ||
        fail "the installed Quint CLI $installed runs the evaluator '$const' (QUINT_EVALUATOR_VERSION), but $pin pins evaluator=$evaluator; see $howto"
    grep -Fq "'$asset'" "$bm" ||
        fail "$bm does not name the asset $asset for this platform, so the CLI would fetch another archive than the one pinned; see $howto"
    grep -Fq "$api" "$bm" ||
        fail "$bm does not read its evaluator from $api, the release this pin was taken from; see $howto"
    # Where the CLI looks for the executable: config.js joins QUINT_HOME and
    # rust-evaluator-<version>, and binaryManager.js joins that and
    # quint_evaluator. Every line naming these must be one 0.34.0 has, so a CLI
    # that changes them fails here rather than passing on a stamped directory
    # it never reads; one that looks elsewhere through other lines is not seen.
    exact_lines "$cli/dist/src/config.js" 'rustEvaluatorDir|quintConfigDir|QUINT_HOME' "exports.rustEvaluatorDir = rustEvaluatorDir;
 * QUINT_HOME environment variable.
function quintConfigDir() {
    return process.env.QUINT_HOME ?? path_1.default.join(os_1.default.homedir(), '.quint');
    return path_1.default.join(quintConfigDir(), \`apalache-dist-\${version}\`);
function rustEvaluatorDir(version) {
    return path_1.default.join(quintConfigDir(), \`rust-evaluator-\${version}\`);"
    exact_lines "$bm" 'rustEvaluatorDir|executable = |executablePath = ' "    const evaluatorDir = (0, config_1.rustEvaluatorDir)(version);
    const executablePath = path_1.default.join(evaluatorDir, executable);
    const evaluatorDir = (0, config_1.rustEvaluatorDir)(version);
    const executablePath = path_1.default.join(evaluatorDir, executable);
    const executablePath = path_1.default.join(evaluatorDir, executable);
    let executable = 'quint_evaluator';
            executable = 'quint_evaluator.exe';
    const evaluatorDir = (0, config_1.rustEvaluatorDir)(version);"
}

# exact_lines <file> <ERE> <lines>: the lines of the file matching the ERE must
# be exactly <lines>, in order, or the evaluator's directory is not the one
# this check knows.
exact_lines() {
    found=$(grep -E -- "$2" "$1") ||
        fail "cannot find the lines that build the evaluator's path in $1; read it by hand ($howto)"
    [ "$found" = "$3" ] ||
        fail "$1 builds the evaluator's path on lines other than the ones this check knows, so the CLI may look for it outside .quint/rust-evaluator-<version>/quint_evaluator; read it by hand ($howto)"
}


paths() {
    qh=.quint
    dir=$qh/rust-evaluator-$evaluator
    exe=$dir/quint_evaluator
    stamp=$dir/.pachinko-stamp
    for p in "$qh" "$dir"; do
        if [ -L "$p" ]; then
            fail "$p is a symlink; it must be a directory inside this worktree. Remove the symlink itself (not its target), then run just install-quint-evaluator"
        fi
        if [ -e "$p" ] && [ ! -d "$p" ]; then
            fail "$p is not a directory. Remove it, then run just install-quint-evaluator"
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

# The skip test. Returns 1, with the reason in `reason`, unless the directory is
# the one the stamp records, from this platform's pinned archive, with an
# executable quint_evaluator. Called only as a condition, where set -e is off,
# so every test returns.
stamped_ok() {
    if [ ! -e "$dir" ]; then
        reason="no stamp: there is no directory at $dir"
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
            reason="stamp archive mismatch: $stamp does not record the sha256 $pin pins for $plat, $sha"
            return 1
            ;;
    esac
    if ! tree_digest "$dir"; then
        reason="walk failed: the directory $dir cannot be digested"
        return 1
    fi
    if [ "$s" != "archive=$sha${nl}tree=$d" ]; then
        reason="tree digest mismatch: the files under $dir are not the ones $stamp records"
        return 1
    fi
    # $(cat) drops trailing newlines, so the bytes are compared as well: the
    # stamp is exactly the two lines the installer writes.
    if ! printf 'archive=%s\ntree=%s\n' "$sha" "$d" | cmp -s - "$stamp"; then
        reason="stamp malformed: $stamp is not exactly the two lines the installer writes"
        return 1
    fi
    if ! python -I -B -c "$entries_program" "$dir" .pachinko-stamp quint_evaluator; then
        reason="unexpected entries: $dir holds more than the stamp and quint_evaluator (above)"
        return 1
    fi
    if [ -L "$exe" ] || [ ! -f "$exe" ] || [ ! -x "$exe" ]; then
        reason="executable not a regular executable file: $exe"
        return 1
    fi
    return 0
}

# The member check, on the archive $1, before anything is unpacked. It reads
# each member's header with tarfile, which unpacks nothing, so a member's type
# is known before tar could create it. Both v0.8.0 archives hold one regular
# file, quint_evaluator, at the root, and nothing else is accepted. tarfile
# stops without an error at a malformed header after the first, which BSD tar
# also lists and unpacks with status 0, so everything after the last member's
# data must be the end-of-archive blocks: at least two, and nothing but zeros.
members_program='
import gzip, io, sys, tarfile, zlib

def fail(message):
    sys.stderr.write("archive check: " + message + "\n")
    sys.exit(1)

names = []
try:
    with open(sys.argv[1], "rb") as handle:
        raw = gzip.decompress(handle.read())
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:") as archive:
        end = 0
        for member in archive:
            end = member.offset_data + -(-member.size // 512) * 512
            name = member.name
            if name.startswith("/"):
                fail("absolute name " + repr(name))
            if any(part in ("", ".", "..") for part in name.split("/")):
                fail("unsafe component in " + repr(name))
            if not member.isreg():
                kind = "symbolic link" if member.issym() else "hard link" if member.islnk() else "directory" if member.isdir() else "type " + repr(member.type)
                fail("not a regular file: " + repr(name) + " (" + kind + ")")
            if name != "quint_evaluator":
                fail("unexpected member " + repr(name))
            names.append(name)
except (OSError, EOFError, zlib.error, tarfile.TarError) as error:
    fail("cannot read the archive: " + str(error))
if not names:
    fail("the archive is empty")
if len(names) > 1:
    fail("duplicate member quint_evaluator")
tail = raw[end:]
if len(tail) < 1024 or tail.count(0) != len(tail):
    fail("%d bytes after the last member are not the end-of-archive blocks (a malformed or truncated header)" % len(tail))
print("\n".join(names))
'

# Prints the archive's member names, one per line, or fails.
check_members() {
    python -I -B -c "$members_program" "$1"
}

# GNU tar reads TAR_OPTIONS, BSD tar TAR_READER_OPTIONS and TAR_WRITER_OPTIONS,
# and both TAPE; any of them could change what is listed or unpacked.
safe_tar() {
    env -u TAR_OPTIONS -u TAR_READER_OPTIONS -u TAR_WRITER_OPTIONS -u TAPE tar "$@"
}

# The directory $1 must hold exactly the entries $2..., every one a regular
# file, and nothing else, not even an empty directory, which the tree digest
# does not see. The installer runs it on the unpacked archive with
# quint_evaluator, and the skip test on the installed directory with the stamp
# as well.
entries_program='
import os, stat, sys
root = sys.argv[1]
want = sorted(sys.argv[2:])
found = []
for directory, dirs, files in os.walk(root, onerror=lambda e: sys.exit("entry check: " + str(e))):
    for name in dirs + files:
        found.append(os.path.relpath(os.path.join(directory, name), root).replace(os.sep, "/"))
if sorted(found) != want:
    sys.exit("entry check: " + root + " holds " + repr(sorted(found)) + ", not " + repr(want))
for name in want:
    if not stat.S_ISREG(os.lstat(os.path.join(root, name)).st_mode):
        sys.exit("entry check: " + name + " is not a regular file")
'

check_mode() {
    home
    if [ $# -eq 0 ]; then
        why="there are no arguments, so the CLI starts the REPL; the guard does not read --backend"
    else
        case $1 in
            parse | typecheck | compile | docs) exit 0 ;;
        esac
        if [ $# -eq 1 ]; then
            case $1 in
                --help | --version) exit 0 ;;
            esac
        fi
        case $1 in
            run | test | repl)
                why="the first argument is '$1', which runs the evaluator; the guard does not read --backend, so --backend typescript is refused too"
                ;;
            *)
                why="the first argument '$1' is not one of parse, typecheck, compile or docs, so the command may run the evaluator (a verify that reports a violation or an observation, or the REPL Quint starts by default)"
                ;;
        esac
    fi
    read_pin
    platform
    check_cli
    paths
    stamped_ok ||
        fail "the Rust evaluator $evaluator is not installed as $pin pins it ($reason); run just install-quint-evaluator. Checked because $why"
    exit 0
}

install_mode() {
    home
    read_pin
    platform
    check_cli
    paths
    if stamped_ok; then
        say "the Rust evaluator $evaluator ($plat) is installed and matches its stamp; nothing to do."
        exit 0
    fi
    say "installing the Rust evaluator $evaluator ($plat) into $dir: $reason"

    mkdir -p "$qh"
    paths
    # The traps go first, so a signal right after mktemp still removes its directory.
    tmp=''
    trap '[ -z "$tmp" ] || rm -rf -- "${tmp:?}"' EXIT
    trap 'exit 1' HUP INT TERM
    tmp=$(mktemp -d "$qh/install-quint-evaluator.XXXXXX")

    url=https://github.com/quint-co/quint/releases/download/evaluator/$evaluator/$asset
    say "downloading $url"
    curl -fsSL --proto '=https' --proto-redir '=https' --tlsv1.2 -o "$tmp/$asset" "$url" ||
        fail "the download of $url failed"
    sha256_of "$tmp/$asset" || fail "cannot hash the downloaded archive"
    [ "$h" = "$sha" ] ||
        fail "checksum mismatch for $url: $pin pins sha256.$plat=$sha, the download has $h; nothing was installed"

    listed=$(check_members "$tmp/$asset") || fail "the archive's members are not the one regular file quint_evaluator (above); nothing was installed"
    # tar must read the same member names tarfile did, so a difference in names
    # fails here, before anything is unpacked. A difference in a member's type
    # is not seen here: it is caught after unpacking, inside $tmp, by the entry
    # and link-count checks below, before anything moves into place.
    names=$(safe_tar -tzf "$tmp/$asset") || fail "cannot list the archive"
    [ "$names" = "$listed" ] ||
        fail "tar lists the archive as '$names', not '$listed' as the member check read it; nothing was installed"
    mkdir -- "$tmp/new"
    safe_tar -xzf "$tmp/$asset" -C "$tmp/new" || fail "cannot unpack the archive"
    python -I -B -c "$entries_program" "$tmp/new" quint_evaluator ||
        fail "the unpacked directory does not match the archive's listing"
    [ -f "$tmp/new/quint_evaluator" ] && [ ! -L "$tmp/new/quint_evaluator" ] && [ -x "$tmp/new/quint_evaluator" ] ||
        fail "the archive holds no executable quint_evaluator"
    # A hard link is a regular file to the entry check; a tar that unpacked one
    # where tarfile read a regular member would leave a second name for another
    # file, so the unpacked executable must have one link only.
    [ "$(find "$tmp/new/quint_evaluator" -links 1)" = "$tmp/new/quint_evaluator" ] ||
        fail "the unpacked quint_evaluator is a hard link to another file; nothing was installed"
    tree_digest "$tmp/new" || fail "cannot digest the unpacked directory"
    printf 'archive=%s\ntree=%s\n' "$sha" "$d" >|"$tmp/new/.pachinko-stamp"

    paths
    if [ -e "$dir" ] || [ -L "$dir" ]; then
        mv -- "$dir" "$tmp/old"
    fi
    if [ -e "$dir" ] || [ -L "$dir" ]; then
        fail "$dir reappeared while installing, so another process is writing it; run just install-quint-evaluator again once it has finished"
    fi
    mv -- "$tmp/new" "$dir"
    stamped_ok ||
        fail "the installed directory fails its own check ($reason); another process may have written it; run just install-quint-evaluator again"
    # Under this script's set -e a failing rm in the EXIT trap would still make
    # the exit status 1 (probed under dash), but with no message saying the
    # install is in place. So the success path removes the temporary directory
    # itself, names what is left if that fails, and checks it is gone. The trap
    # stays for every failure path.
    rm -rf -- "${tmp:?}" ||
        fail "installed $dir, but cannot remove the temporary directory $tmp, which holds the archive and the old directory; remove it by hand"
    [ ! -e "$tmp" ] ||
        fail "installed $dir, but the temporary directory $tmp is still there; remove it by hand"
    tmp=''
    say "installed the Rust evaluator $evaluator ($plat, sha256 $sha) into $dir."
}

case $mode in
    check) check_mode "$@" ;;
    # The script has moved to the repository root, so a relative name is read
    # from the directory it was called in.
    archive)
        case $1 in
            /*) archive=$1 ;;
            *) archive=$start/$1 ;;
        esac
        check_members "$archive" >/dev/null || fail "the archive $1 fails the member check (above)"
        ;;
    *) install_mode ;;
esac
