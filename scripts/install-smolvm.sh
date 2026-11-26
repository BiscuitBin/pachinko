#!/bin/sh
PATH=/usr/bin:/bin:/usr/sbin:/sbin
export PATH
# smolvm, the microVM that runs Quint Studio (decision 0022), from the release
# tarball tools/smolvm/smolvm.txt pins by SHA-256, unpacked to .tools/smolvm/.
# macOS on arm64 only. Two modes:
#
# - No argument (`just install-smolvm`): install. It refuses another host, a pin
#   that fails scripts/check-smolvm-pin.sh, and a symlink or non-directory at
#   .tools or .tools/smolvm. It takes the install lock (below), then skips, with
#   no network, only when the skip test passes. Any other tree is replaced: the
#   tarball is downloaded into a fresh .tools/install-smolvm.* directory, its
#   hash checked, every member name checked, unpacked with the archive's modes,
#   compared entry by entry with the archive's members as Python's tarfile reads
#   them, checked against its own checksums.txt and its signature, stamped there,
#   and only then moved into place, the old tree moved aside into the temporary
#   directory first. An interrupted run leaves the old tree or none, never an
#   unstamped tree that looks finished. The only path ever removed is the
#   temporary directory.
# - `--check` (the launcher's and the purge's guard): offline and read-only. The
#   same refusals, then the skip test; exit 0 only when it passes, else exit 1
#   naming just install-smolvm. It writes nothing and takes no lock.
#
# The skip test (stamped_ok) is one predicate, for the install's skip, its check
# after the move and --check: .tools/smolvm/.pachinko-stamp is a regular file
# holding exactly `archive=<the pinned sha256>` and `tree=<digest>`; `smolvm` (the
# bash launcher that is run) and `smolvm-bin` (the Mach-O it execs) are regular,
# executable files; the digest of the tree now equals tree=; and smolvm-bin
# passes `codesign --verify --strict` and its entitlements, read with
# `codesign -d --entitlements - --xml`, hold com.apple.security.hypervisor as the
# boolean true. The digest is the SHA-256 of the sorted records of every entry
# under the tree, each record its fields joined by NUL and ended by a newline:
# `f`, mode, content SHA-256, path for a regular file; `l`, link target, path for
# a symbolic link (never followed); `d`, mode, path for a directory; the mode is
# the permission bits in octal. Any other type, or a newline in a path or a
# target, fails the walk. It leaves out the stamp, and exactly the two top-level
# templates smolvm expands beside its binary on first use, overlay-default.ext4
# and storage-template.ext4, when each is a regular file, so a first boot leaves
# the skip test passing; the stamp does not vouch for those two files. Their
# repair: removing .tools/smolvm/overlay-default.ext4 and storage-template.ext4
# makes smolvm expand them again on its next use, and --check still passes. The
# stamp detects drift and damage in a tree this script stamped, not a stamp
# rewritten by hand. An archive member named .pachinko-stamp is refused, and the
# stamp is made with O_CREAT|O_EXCL|O_NOFOLLOW, so no member can redirect it.
# Every SHA-256 here (the download's and checksums.txt's) is computed by the pixi
# Python's hashlib, not by /usr/bin/shasum, a Perl script that reads PERL5OPT
# and PERL5LIB; curl runs with -q, so it reads no ~/.curlrc. The script reads
# the binary's signature and never runs the binary, never signs it, and never
# removes a signature or an extended attribute.
#
# The install lock, .tools/smolvm.lock, serialises an install with every user of
# the tree. Its protocol, which scripts/smolvm.sh follows too: the name is checked
# without following it (a regular file, this user's, one link); an absent one is
# made by the pixi Python with O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW and mode 0600
# and checked again; it is opened read-only on descriptor 9 (`exec 9<`, which
# never creates or writes), locked with `/usr/bin/lockf -s -t 0 9`, refused at
# once when busy (lockf exits 75), and the opened descriptor is checked to be the
# name's file (regular, this user's, one link, the same device and inode). The
# lock may hold no ACL entry that allows anything (read from /bin/ls -lde). The
# install mode holds it from before its skip test until it exits. `just smolvm`,
# `just smolvm-purge` and the orphan's entry point take it before they run
# --check and hold it until they end, so no install moves the tree under them.
# Whoever takes both takes the install lock first and the state lock second,
# never the other way round. --check neither takes nor needs it, since its
# callers hold it. No tool removes its entry.
#
# PATH is fixed on the first line and an exported shell function refused on the
# next, before any other command runs, since /bin/sh imports one at start-up in
# place of a command or a non-special builtin. The root is found with builtins
# only; every helper is named by its absolute path, Python by the pixi path.
/usr/bin/env | /usr/bin/grep -q '^BASH_FUNC_'
case $? in
    1) ;;
    0)
        /usr/bin/printf '%s\n' 'install-smolvm: refused: the environment exports a shell function (BASH_FUNC_...), which /bin/sh and smolvm'\''s bash launcher would import in place of a command; unset it, then run again' >&2
        exit 1
        ;;
    *)
        /usr/bin/printf '%s\n' 'install-smolvm: refused: cannot read the environment to check it for an exported shell function' >&2
        exit 1
        ;;
esac
set -eu

case $0 in
    */*) here=${0%/*} ;;
    *) here=. ;;
esac
CDPATH='' cd -P -- "$here/.." || exit 1
root=$(pwd -P)
cd -- "$root"

LC_ALL=C
export LC_ALL

pin=tools/smolvm/smolvm.txt
tools=.tools
tree=.tools/smolvm
lock=.tools/smolvm.lock
py=$root/.pixi/envs/default/bin/python
nl='
'

mode=install
if [ $# -gt 0 ]; then
    if [ $# -eq 1 ] && [ "$1" = --check ]; then
        mode=check
    else
        printf '%s\n' 'usage: install-smolvm.sh [--check]' >&2
        exit 2
    fi
fi

fail() {
    if [ "$mode" = check ]; then
        printf 'install-smolvm --check: refused: %s\n' "$*" >&2
    else
        printf 'install-smolvm: %s\n' "$*" >&2
    fi
    exit 1
}

say() {
    printf 'install-smolvm: %s\n' "$*"
}

host="$(/usr/bin/uname -s) $(/usr/bin/uname -m)"
[ "$host" = 'Darwin arm64' ] ||
    fail "smolvm and Quint Studio run on macOS on arm64 only (ruling 4, decision 0022); this host is $host"

status=0
/bin/sh scripts/check-smolvm-pin.sh || status=$?
[ "$status" -eq 0 ] ||
    fail "$pin fails its check (scripts/check-smolvm-pin.sh exit $status, above); fix the pin, then run just install-smolvm"
version=$(/usr/bin/sed -n 's/^version=//p' "$pin")
asset=$(/usr/bin/sed -n 's/^asset=//p' "$pin")
sha=$(/usr/bin/sed -n 's/^sha256=//p' "$pin")
top=${asset%.tar.gz}

paths() {
    for p in "$tools" "$tree"; do
        if [ -L "$p" ]; then
            fail "$p is a symlink; it must be a directory inside this worktree. Remove the symlink itself (not its target), then run just install-smolvm"
        fi
        if [ -e "$p" ] && [ ! -d "$p" ]; then
            fail "$p is not a directory. Remove it, then run just install-smolvm"
        fi
    done
}

python_ok() {
    [ -f "$py" ] && [ -x "$py" ] ||
        fail "pixi's Python, $py, is missing; run just initialize"
}

# One process, so a failed walk or read cannot yield a digest; it exits non-zero
# on any error, on any entry that is not a regular file, a directory or a
# symbolic link, and on a newline in a path or a link target.
digest_program='
import hashlib, os, stat, sys

def fail(message):
    sys.stderr.write("tree digest: " + message + "\n")
    sys.exit(1)

def raise_error(error):
    raise error

root = sys.argv[1]
templates = ("overlay-default.ext4", "storage-template.ext4")
records = []
try:
    if not stat.S_ISDIR(os.lstat(root).st_mode):
        fail(root + " is not a directory")
    for directory, dirs, files in os.walk(root, onerror=raise_error):
        for name in dirs + files:
            path = os.path.join(directory, name)
            rel = os.path.relpath(path, root)
            if "\n" in rel:
                fail("unsupported path " + repr(rel))
            if rel == ".pachinko-stamp":
                continue
            st = os.lstat(path)
            mode = format(stat.S_IMODE(st.st_mode), "o").encode()
            key = os.fsencode(rel)
            if stat.S_ISREG(st.st_mode):
                if rel in templates:
                    continue
                digest = hashlib.sha256()
                fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
                with os.fdopen(fd, "rb") as handle:
                    for chunk in iter(lambda: handle.read(1 << 20), b""):
                        digest.update(chunk)
                records.append(b"\0".join([b"f", mode, digest.hexdigest().encode(), key]))
            elif stat.S_ISLNK(st.st_mode):
                target = os.readlink(path)
                if "\n" in target:
                    fail("unsupported link target at " + repr(rel))
                records.append(b"\0".join([b"l", os.fsencode(target), key]))
            elif stat.S_ISDIR(st.st_mode):
                records.append(b"\0".join([b"d", mode, key]))
            else:
                fail("not a regular file, a directory or a symbolic link: " + rel)
except OSError as error:
    fail(str(error))
records.sort()
print(hashlib.sha256(b"".join(record + b"\n" for record in records)).hexdigest())
'

# Sets d to the digest of the tree $1.
tree_digest() {
    d=$("$py" -I -B -c "$digest_program" "$1") || return 1
    case $d in
        *[!0-9a-f]* | '') return 1 ;;
    esac
    [ ${#d} -eq 64 ] || return 1
}

# Exits 0 only when the plist on stdin grants com.apple.security.hypervisor as true.
entitlements_program='
import plistlib, sys
try:
    granted = plistlib.loads(sys.stdin.buffer.read())
except Exception as error:
    sys.stderr.write("entitlements: " + str(error) + "\n")
    sys.exit(1)
sys.exit(0 if isinstance(granted, dict) and granted.get("com.apple.security.hypervisor") is True else 1)
'

# The two codesign reads of the Mach-O $1. Returns 1, with the reason in
# `reason`, unless it is validly signed and holds the Hypervisor entitlement.
signed_ok() {
    if ! /usr/bin/codesign --verify --strict "$1" 2>/dev/null; then
        reason="signature invalid: codesign --verify --strict fails on $1"
        return 1
    fi
    if ! granted=$(/usr/bin/codesign -d --entitlements - --xml "$1" 2>/dev/null); then
        reason="entitlements unreadable: codesign -d --entitlements - fails on $1"
        return 1
    fi
    if ! printf '%s' "$granted" | "$py" -I -B -c "$entitlements_program"; then
        reason="no Hypervisor entitlement: $1 does not hold com.apple.security.hypervisor as true"
        return 1
    fi
    return 0
}

# The skip test. Returns 1, with the reason in `reason`, unless the tree $1 is the
# one its stamp records, from the pinned archive, with an executable launcher and
# binary and a valid signature. Called only as a condition, where set -e is off,
# so every test returns.
stamped_ok() {
    t=$1
    stamp=$t/.pachinko-stamp
    if [ ! -e "$t" ] && [ ! -L "$t" ]; then
        reason="no stamp: there is no tree at $t"
        return 1
    fi
    if [ -L "$t" ] || [ ! -d "$t" ]; then
        reason="$t is not a directory"
        return 1
    fi
    if [ -L "$stamp" ] || [ ! -f "$stamp" ]; then
        reason="no stamp at $stamp"
        return 1
    fi
    if ! s=$(/bin/cat -- "$stamp" 2>/dev/null); then
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
    for x in "$t/smolvm" "$t/smolvm-bin"; do
        if [ -L "$x" ] || [ ! -f "$x" ] || [ ! -x "$x" ]; then
            reason="not executable: $x is not a regular, executable file"
            return 1
        fi
    done
    if ! tree_digest "$t"; then
        reason="walk failed: the tree under $t cannot be digested"
        return 1
    fi
    if [ "$s" != "archive=$sha${nl}tree=$d" ]; then
        reason="tree digest mismatch: the entries under $t are not the ones $stamp records"
        return 1
    fi
    signed_ok "$t/smolvm-bin" || return 1
    return 0
}

# Reads the archive $2 with tarfile (no extraction) and checks every member: a
# name under $3/ with no empty, `.` or `..` component; a regular file, a directory
# or a symbolic link, never setuid or setgid, listed once, under parents the
# archive lists as directories. With `names`, also requires tar's listing on
# stdin to name the same members. With `tree`, requires what tar unpacked under
# $4 to be exactly those members, each with its type, its mode and, for a link,
# its target, and nothing else beside $3/ but the archive.
archive_program='
import os, stat, sys, tarfile

def fail(message):
    sys.stderr.write("archive check: " + message + "\n")
    sys.exit(1)

def raise_error(error):
    raise error

action, archive, top = sys.argv[1], sys.argv[2], sys.argv[3]

def checked(name, source):
    parts = name.split("/")
    if len(parts) > 1 and parts[-1] == "":
        parts = parts[:-1]
    if parts[0] != top or any(part in ("", ".", "..") for part in parts) or "\n" in name or "\\" in name:
        fail("unexpected member " + repr(name) + " in " + source)
    if any(part.lower() == ".pachinko-stamp" for part in parts):
        fail("member " + repr(name) + " in " + source + " takes the stamp'"'"'s name, .pachinko-stamp, which only this installer writes")
    return "/".join(parts)

members = {}
try:
    with tarfile.open(archive, "r:gz") as tar:
        for member in tar.getmembers():
            name = checked(member.name, "the archive")
            if name in members:
                fail("member listed twice: " + name)
            if member.isdir():
                kind, target = "d", None
            elif member.issym():
                kind, target = "l", member.linkname
                if target == "" or "\n" in target:
                    fail("unsupported link target at " + name)
            elif member.isreg():
                kind, target = "f", None
            else:
                fail("member of an unsupported type: " + name)
            if member.mode & 0o6000:
                fail("setuid or setgid member: " + name)
            members[name] = (kind, member.mode & 0o7777, target)
except (OSError, tarfile.TarError) as error:
    fail(str(error))
if not members:
    fail("the archive is empty")
for name in members:
    parts = name.split("/")
    for depth in range(1, len(parts)):
        parent = "/".join(parts[:depth])
        if members.get(parent, ("",))[0] != "d":
            fail(name + " lies under " + parent + ", which the archive does not list as a directory")

if action == "names":
    listed = set()
    for line in sys.stdin.read().split("\n"):
        if line != "":
            listed.add(checked(line, "tar'"'"'s listing"))
    if listed != set(members):
        fail("tar'"'"'s listing and tarfile'"'"'s reading disagree on " + repr(sorted(listed ^ set(members))[:5]))
    sys.exit(0)

base = sys.argv[4]

def entry(path):
    st = os.lstat(path)
    bits = stat.S_IMODE(st.st_mode)
    if stat.S_ISDIR(st.st_mode):
        return ("d", bits, None)
    if stat.S_ISLNK(st.st_mode):
        return ("l", bits, os.readlink(path))
    if stat.S_ISREG(st.st_mode):
        return ("f", bits, None)
    return ("?", bits, None)

found = {}
try:
    extra = set(os.listdir(base)) - {top, os.path.basename(archive)}
    if extra:
        fail("unpacked outside " + top + "/: " + repr(sorted(extra)))
    root = os.path.join(base, top)
    found[top] = entry(root)
    if found[top][0] != "d":
        fail(top + " is not a directory")
    for directory, dirs, files in os.walk(root, onerror=raise_error):
        for name in dirs + files:
            path = os.path.join(directory, name)
            found[os.path.relpath(path, base)] = entry(path)
except OSError as error:
    fail(str(error))
for name in sorted(set(members) | set(found)):
    if members.get(name) != found.get(name):
        fail("the unpacked tree differs from the archive at " + name + ": the archive has " + repr(members.get(name)) + ", the tree " + repr(found.get(name)))
'

# SHA-256 with hashlib. `file <path>` prints the file's digest. `check <list>`
# does what `shasum -a 256 -q -c <list>` does, from the current directory: every
# non-empty line is `<64 hex> <space or *><path>`, at least one line, each file
# opened (following links) and hashed; each failure is named on standard error
# as `<path>: FAILED` or `<path>: FAILED open or read`; exit 1 on any.
sha_program='
import hashlib, re, sys

def digest(path):
    value = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            value.update(chunk)
    return value.hexdigest()

action, path = sys.argv[1], sys.argv[2]
if action == "file":
    try:
        print(digest(path))
    except OSError as error:
        sys.stderr.write("sha256: " + str(error) + "\n")
        sys.exit(1)
    sys.exit(0)
try:
    with open(path, "rb") as handle:
        lines = handle.read().split(b"\n")
except OSError as error:
    sys.stderr.write("sha256: " + str(error) + "\n")
    sys.exit(1)
failed = checked = 0
for number, line in enumerate(lines, 1):
    if line == b"":
        continue
    match = re.fullmatch(rb"([0-9A-Fa-f]{64}) [ *](.+)", line)
    if not match:
        sys.stderr.write(path + ": line " + str(number) + " is not a SHA-256 checksum line\n")
        sys.exit(1)
    name = match.group(2).decode("utf-8", "surrogateescape")
    checked += 1
    try:
        found = digest(name)
    except OSError:
        sys.stderr.write(name + ": FAILED open or read\n")
        failed += 1
        continue
    if found != match.group(1).decode().lower():
        sys.stderr.write(name + ": FAILED\n")
        failed += 1
if checked == 0:
    sys.stderr.write(path + ": no SHA-256 checksum lines\n")
    sys.exit(1)
if failed:
    sys.stderr.write(str(failed) + " of " + str(checked) + " computed checksums did NOT match\n")
    sys.exit(1)
'

# Writes the stamp $1 with $2, made new without following a link.
stamp_program='
import os, sys
fd = os.open(sys.argv[1], os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644)
with os.fdopen(fd, "w") as handle:
    handle.write(sys.argv[2])
'

# Checks the lock's name and, with `held`, that descriptor 9 is its file.
lock_program='
import os, stat, sys

def fail(message):
    sys.stderr.write("install lock: " + message + "\n")
    sys.exit(1)

action, name = sys.argv[1], sys.argv[2]

def checked():
    st = os.lstat(name)
    if not stat.S_ISREG(st.st_mode):
        fail(name + " is not a regular file")
    if st.st_uid != os.getuid():
        fail(name + " is not owned by this user")
    if st.st_nlink != 1:
        fail(name + " has " + str(st.st_nlink) + " links, not 1")
    return st

try:
    if action == "make":
        try:
            os.lstat(name)
        except FileNotFoundError:
            try:
                os.close(os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600))
            except FileExistsError:
                pass
        checked()
    elif action == "held":
        opened = os.fstat(9)
        st = checked()
        if not stat.S_ISREG(opened.st_mode) or opened.st_uid != os.getuid() or opened.st_nlink != 1:
            fail("the descriptor opened on " + name + " is not a regular file of this user with one link")
        if (opened.st_dev, opened.st_ino) != (st.st_dev, st.st_ino):
            fail(name + " was replaced between its check and its open")
    else:
        fail("unknown action " + action)
except OSError as error:
    fail(str(error))
'

# Takes the install lock on descriptor 9, held until this shell exits.
take_lock() {
    "$py" -I -B -c "$lock_program" make "$lock" ||
        fail "the install lock $lock cannot be used (above); if nothing of this worktree's smolvm is running, remove $lock itself, then run just install-smolvm"
    command exec 9<"$lock" || fail "cannot open the install lock $lock"
    status=0
    /usr/bin/lockf -s -t 0 9 || status=$?
    case $status in
        0) ;;
        75) fail "the install lock $lock is held by another process: another just install-smolvm, or a just smolvm or just smolvm-purge in this worktree. Wait for it to end, then run just install-smolvm again" ;;
        *) fail "cannot take the install lock $lock (lockf exit $status)" ;;
    esac
    "$py" -I -B -c "$lock_program" held "$lock" ||
        fail "the install lock $lock fails its check (above); run just install-smolvm again"
    # No ACL entry may allow anything, as the launcher's locks (scripts/smolvm.sh).
    acl=$(/bin/ls -lde -- "$lock") || fail "cannot read the ACL of the install lock $lock"
    case $acl in
        *' allow '*) fail "the install lock $lock has an ACL entry that allows access beyond its mode bits; remove it (chmod -N $lock), then run just install-smolvm" ;;
    esac
}

# GNU tar reads TAR_OPTIONS, BSD tar TAR_READER_OPTIONS and TAR_WRITER_OPTIONS,
# and both TAPE; any of them could change what is listed or unpacked.
untar() {
    /usr/bin/env -u TAR_OPTIONS -u TAR_READER_OPTIONS -u TAR_WRITER_OPTIONS -u TAPE /usr/bin/tar "$@"
}

check_mode() {
    paths
    python_ok
    stamped_ok "$tree" ||
        fail "smolvm $version is not installed as $pin pins it ($reason); run just install-smolvm"
    exit 0
}

install_mode() {
    paths
    python_ok
    [ -d "$tools" ] || /bin/mkdir -- "$tools" 2>/dev/null || :
    paths
    [ -d "$tools" ] || fail "cannot make $tools"
    take_lock
    paths
    if stamped_ok "$tree"; then
        say "smolvm $version is installed and matches its stamp; nothing to do."
        exit 0
    fi
    say "installing smolvm $version into $tree: $reason"

    # The traps go first, so a signal right after mktemp still removes its directory.
    tmp=''
    trap '[ -z "$tmp" ] || /bin/rm -rf -- "${tmp:?}"' EXIT
    trap 'exit 1' HUP INT TERM
    tmp=$(/usr/bin/mktemp -d "$tools/install-smolvm.XXXXXX")

    url=https://github.com/smol-machines/smolvm/releases/download/v$version/$asset
    say "downloading $url"
    /usr/bin/curl -q -fsSL --proto '=https' --proto-redir '=https' --tlsv1.2 -o "$tmp/$asset" "$url" ||
        fail "the download of $url failed"
    h=$("$py" -I -B -c "$sha_program" file "$tmp/$asset") || fail "cannot hash the downloaded archive"
    [ "$h" = "$sha" ] ||
        fail "checksum mismatch for $url: $pin pins sha256 $sha, the download has $h; nothing was installed"

    list=$(untar -tzf "$tmp/$asset") || fail "cannot list the archive"
    printf '%s\n' "$list" | "$py" -I -B -c "$archive_program" names "$tmp/$asset" "$top" ||
        fail "the archive's members fail their check (above); nothing was unpacked"
    untar -xpzf "$tmp/$asset" -C "$tmp" || fail "cannot unpack the archive"
    "$py" -I -B -c "$archive_program" tree "$tmp/$asset" "$top" "$tmp" ||
        fail "the unpacked tree does not match the archive's members (above)"

    new=$tmp/$top
    for f in smolvm smolvm-bin; do
        [ -f "$new/$f" ] && [ ! -L "$new/$f" ] && [ -x "$new/$f" ] ||
            fail "the archive holds no executable $top/$f"
    done
    for f in checksums.txt lib/libkrun.dylib lib/libkrunfw.5.dylib lib/libkrunfw.dylib lib/libepoxy.0.dylib lib/libMoltenVK.dylib lib/libvirglrenderer.1.dylib overlay-default.ext4.zst overlay-template.ext4.zst storage-template.ext4.zst; do
        [ -f "$new/$f" ] || fail "the archive holds no $top/$f, which smolvm needs beside its binary"
    done
    [ -d "$new/agent-rootfs" ] && [ ! -L "$new/agent-rootfs" ] ||
        fail "the archive holds no $top/agent-rootfs/ directory"
    (cd -- "$new" && "$py" -I -B -c "$sha_program" check checksums.txt) ||
        fail "the unpacked files fail the release's own checksums.txt (above)"
    signed_ok "$new/smolvm-bin" || fail "$reason; nothing was installed"
    tree_digest "$new" || fail "cannot digest the unpacked tree"
    "$py" -I -B -c "$stamp_program" "$new/.pachinko-stamp" "archive=$sha${nl}tree=$d$nl" ||
        fail "cannot write the stamp $new/.pachinko-stamp (above)"

    paths
    if [ -e "$tree" ] || [ -L "$tree" ]; then
        /bin/mv -- "$tree" "$tmp/old"
    fi
    if [ -e "$tree" ] || [ -L "$tree" ]; then
        fail "$tree reappeared while installing, so another process is writing it; run just install-smolvm again once it has finished"
    fi
    /bin/mv -- "$new" "$tree"
    stamped_ok "$tree" ||
        fail "the installed tree fails its own check ($reason); another process may have written it; run just install-smolvm again"
    say "installed smolvm $version (sha256 $sha) into $tree."
}

if [ "$mode" = check ]; then
    check_mode
else
    install_mode
fi
