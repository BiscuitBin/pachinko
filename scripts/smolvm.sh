#!/bin/sh
PATH=/usr/bin:/bin:/usr/sbin:/sbin
export PATH
/usr/bin/env | /usr/bin/grep -q '^BASH_FUNC_'
case $? in
    1) ;;
    0)
        /usr/bin/printf '%s\n' 'smolvm launcher: refused: the environment exports a shell function (BASH_FUNC_...), which /bin/sh would run in place of a command of this script; unset it, then run again' >&2
        exit 1
        ;;
    *)
        /usr/bin/printf '%s\n' 'smolvm launcher: refused: cannot read the environment to check it for an exported shell function' >&2
        exit 1
        ;;
esac
# The launcher for the stamped smolvm (decision 0022). `just smolvm <args>` runs
# `run`, `just smolvm-purge` runs `purge`; QS04's and QS05's scripts call this file
# directly, without `just`, as `/bin/sh <path>/scripts/smolvm.sh <mode> ...` or as
# the executable it is. Each mode is one shell process that traps HUP, INT and TERM
# before its first lock, so it always waits for a smolvm it started and holds its
# locks until it ends; then it checks, then acts:
#
# run <smolvm arguments>
#   0. PATH is fixed on the first line and an exported shell function is refused on
#      the next (as is an environment that cannot be read), before any command a
#      function could replace; the root is found from $0 with builtins; every helper
#      is named by absolute path, Python as pixi's.
#   1. The install lock, .tools/smolvm.lock, taken with lockf(1) on a read-only
#      descriptor (8) and refused at once when busy; then
#      `scripts/install-smolvm.sh --check`, so no `just install-smolvm` can replace
#      the tree between the check and the run.
#   2. Refusals of what the caller set that would move smolvm's state or change what
#      it loads: any SMOLVM_*, DYLD_*, XDG_CACHE_HOME, XDG_DATA_HOME, XDG_CONFIG_HOME
#      or DOCKER_CONFIG variable, a HOME other than the user database's, and
#      --state-dir before `--`.
#   3. The state lock, <cache>/<name>-<hash>.lock beside the state directory, on
#      descriptor 9, after the parents are checked; refused at once when busy, the
#      refusal naming the processes that hold it.
#   4. Under both locks: the parents from the home directory down, of which
#      Library/Caches/pachinko and its smolvm/ must have mode 0700 exactly; the state
#      directory, made when absent with mode 0700, then at once its marker, then
#      tmp/ and docker-config/ (an existing state directory that is wholly empty, an
#      interrupted creation, is removed and made again; a missing tmp/ or
#      docker-config/ under a valid marker is made again); the marker, which holds
#      path= (absolute), id= (32 lowercase hex) and pin= (64 lowercase hex) once
#      each and nothing else; the worktree's id file .tools/smolvm-state-id; the
#      state link /tmp/pachinko-smolvm-<hash>, made again when a restart has removed
#      it; and no symbolic link anywhere under the state directory.
#   5. A sentinel line on standard error, then smolvm, with a cleared environment
#      whose HOME, TMPDIR, DOCKER_CONFIG and SMOLVM_DATA_DIR are the state link and
#      directories inside the state, and with both lock descriptors closed for it.
#      Since the shell traps HUP, INT and TERM and waits for smolvm, only kill -9 of
#      this shell alone frees the locks before smolvm ends.
#
# purge
#   Steps 0 to 4 without creating state (a wholly empty state directory is removed);
#   with no state, says so and exits 0. Then smolvm's own listing
#   (`machine ls --json`), which must succeed and list no machine; then a walk that
#   every entry under the state directory is on its own filesystem; then the state
#   directory is renamed to <cache>/.purging-<name>-<pid>, which must be the same
#   directory (device and inode) still holding the marker, the state link is
#   removed, and the renamed directory is removed. A removal that fails part way
#   leaves only that .purging-* directory, which nothing here uses and which is
#   removed by hand. The lock entry, the parents and the id file stay.
#
# purge --orphan <name> [--delete <machine>]
#   The same for a state directory, <cache>/<name>, whose marker names a worktree
#   that no longer exists or no longer holds the marker's id; refused when it does.
#   --delete removes one machine that the orphan's listing has just listed, matched
#   exactly against the listed names; an empty name or one starting with - is
#   refused.
#
# Under a marker that records another pin, only `machine ls`, `machine stop` and
# `machine delete` run, and the purge accepts the state, so a worktree that
# received a moved pin can empty and remove its old state. Nothing here ever
# removes a lock entry. smolvm's standard output is smolvm's alone: everything this
# script prints goes to standard error.
set -eu

case $0 in
    */*) here=${0%/*} ;;
    *) here=. ;;
esac
CDPATH='' cd -- "$here/.." || exit 1
root=$(pwd -P)
LC_ALL=C
export LC_ALL

py=$root/.pixi/envs/default/bin/python
pin_file=tools/smolvm/smolvm.txt
install_lock=$root/.tools/smolvm.lock
id_file=$root/.tools/smolvm-state-id
release=$root/.tools/smolvm/smolvm
sentinel='smolvm launcher: running the stamped smolvm'
link_limit=42
nl='
'

say() {
    /usr/bin/printf 'smolvm launcher: %s\n' "$*" >&2
}

refuse() {
    /usr/bin/printf 'smolvm launcher: refused: %s\n' "$*" >&2
    exit 1
}

[ -f "$py" ] && [ -x "$py" ] ||
    refuse "pixi's Python, $py, is missing; run just initialize (or just sync)"

# The checks that need lstat, fstat, the user database, O_EXCL or JSON, in one
# program run as `python -I -B -c "$helper" <command> <arguments>`. Each command
# exits 0 when the check holds, 1 with its reason on standard error when it does
# not, and 2 on an unexpected error, which callers treat as a refusal too.
helper='
import errno, hashlib, json, os, pwd, re, stat, sys

def no(message):
    sys.stderr.write(message + "\n")
    sys.exit(1)

def owned(st):
    return st.st_uid == os.getuid()

def ident(path):
    base = os.path.basename(path).lower()
    name = re.sub(r"[^a-z0-9-]", "-", base)
    digest = hashlib.sha256(path.encode("utf-8", "surrogateescape")).hexdigest()[:8]
    return name, digest

command, args = sys.argv[1], sys.argv[2:]
try:
    if command == "home":
        print(pwd.getpwuid(os.getuid()).pw_dir)
    elif command == "ident":
        name, digest = ident(args[0])
        print(name + "-" + digest)
        print(digest)
    elif command == "lexists":
        try:
            os.lstat(args[0])
        except FileNotFoundError:
            print("absent")
        else:
            print("present")
    elif command == "dir":
        path, exact = args[0], args[1] if len(args) > 1 else ""
        st = os.lstat(path)
        if stat.S_ISLNK(st.st_mode):
            no(path + " is a symbolic link")
        if not stat.S_ISDIR(st.st_mode):
            no(path + " is not a directory")
        if not owned(st):
            no(path + " is not owned by you")
        mode = stat.S_IMODE(st.st_mode)
        if exact and mode != int(exact, 8):
            no(path + " has mode %04o, not %s; run chmod %s %s" % (mode, exact, exact.lstrip("0"), path))
        if mode & 0o022:
            no(path + " is writable by its group or by others (mode %04o)" % mode)
    elif command == "devino":
        st = os.lstat(args[0])
        print("%d:%d" % (st.st_dev, st.st_ino))
    elif command == "empty":
        if os.listdir(args[0]):
            no(args[0] + " is not empty")
    elif command == "file":
        st = os.lstat(args[0])
        if stat.S_ISLNK(st.st_mode) or not stat.S_ISREG(st.st_mode):
            no(args[0] + " is not a regular file")
        if not owned(st):
            no(args[0] + " is not owned by you")
        if st.st_nlink != 1:
            no(args[0] + " has %d links, not 1" % st.st_nlink)
    elif command == "mklock":
        try:
            fd = os.open(args[0], os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        except FileExistsError:
            pass
        else:
            os.close(fd)
    elif command == "fd":
        path, number = args[0], int(args[1])
        held, named = os.fstat(number), os.lstat(path)
        if not stat.S_ISREG(held.st_mode) or not owned(held) or held.st_nlink != 1:
            no("the descriptor locked is not a regular file of yours with one link")
        if stat.S_ISLNK(named.st_mode) or not stat.S_ISREG(named.st_mode):
            no(path + " is no longer a regular file")
        if (held.st_dev, held.st_ino) != (named.st_dev, named.st_ino):
            no(path + " was replaced between its check and its lock")
    elif command == "write-new":
        # Written whole to <path>.new and fsynced, then linked into place, which fails
        # if the name exists: an interruption leaves only <path>.new, never a partial
        # file at <path>.
        path, staged = args[0], args[0] + ".new"
        fd = os.open(staged, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "w") as handle:
            handle.write(args[1])
            handle.flush()
            os.fsync(handle.fileno())
        os.link(staged, path, follow_symlinks=False)
        os.unlink(staged)
    elif command == "interrupted":
        # The state directory holds nothing, or only a marker that was never linked
        # into place (<marker>.new): a creation interrupted before its marker.
        path = args[0]
        names = os.listdir(path)
        if any(name != ".pachinko-state.new" for name in names):
            no(path + " is not an interrupted creation")
        for name in names:
            st = os.lstat(os.path.join(path, name))
            if not stat.S_ISREG(st.st_mode):
                no(os.path.join(path, name) + " is not a regular file")
            os.unlink(os.path.join(path, name))
    elif command == "symlink":
        os.symlink(args[0], args[1])
    elif command == "link":
        path, target = args[0], args[1]
        st = os.lstat(path)
        if not stat.S_ISLNK(st.st_mode):
            no(path + " is not a symbolic link")
        if not owned(st):
            no(path + " is not owned by you")
        if os.readlink(path) != target:
            no(path + " points to " + os.readlink(path) + ", not " + target)
    elif command == "marker":
        fd = os.open(args[0], os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd, "rb") as handle:
            text = handle.read().decode("utf-8", "surrogateescape")
        if not text.endswith("\n"):
            no(args[0] + " does not end with a newline")
        fields = {}
        for line in text[:-1].split("\n"):
            key, sep, value = line.partition("=")
            if not sep or key not in ("path", "id", "pin"):
                no(args[0] + " holds a line that is not path=, id= or pin=: " + repr(line)[:200])
            if key in fields:
                no(args[0] + " holds " + key + "= more than once")
            fields[key] = value
        for key in ("path", "id", "pin"):
            if key not in fields:
                no(args[0] + " lacks " + key + "=")
        if not fields["path"].startswith("/") or "\0" in fields["path"]:
            no(args[0] + " records a path= that is not absolute")
        if not re.fullmatch(r"[0-9a-f]{32}", fields["id"]):
            no(args[0] + " records an id= that is not 32 lowercase hex digits")
        if not re.fullmatch(r"[0-9a-f]{64}", fields["pin"]):
            no(args[0] + " records a pin= that is not 64 lowercase hex digits")
        print(fields["id"])
        print(fields["pin"])
        print(fields["path"])
    elif command == "id":
        fd = os.open(args[0], os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd, "rb") as handle:
            text = handle.read().decode("ascii", "replace")
        if not re.fullmatch(r"[0-9a-f]{32}\n", text):
            no(args[0] + " does not hold one line of 32 lowercase hex digits")
        print(text[:-1])
    elif command == "one-fs":
        root = args[0]
        device = os.lstat(root).st_dev
        def raise_error(error):
            raise error
        for directory, dirs, files in os.walk(root, onerror=raise_error):
            for name in dirs + files:
                path = os.path.join(directory, name)
                if os.lstat(path).st_dev != device:
                    no(path + " is on another filesystem than " + root + " (a mount point under the state directory); unmount it, then run again")
    elif command == "aside":
        source, dest, recorded = args[0], args[1], args[2]
        try:
            os.lstat(dest)
        except FileNotFoundError:
            pass
        else:
            no(dest + " already exists, so " + source + " was not moved and nothing was removed")
        os.rename(source, dest)
        st = os.lstat(dest)
        if not stat.S_ISDIR(st.st_mode) or "%d:%d" % (st.st_dev, st.st_ino) != recorded:
            no("moved " + source + " to " + dest + ", but what was moved is not the state directory that was checked (its device and inode differ); nothing was removed. Look at both paths and put back by hand what belongs where")
        try:
            marked = stat.S_ISREG(os.lstat(os.path.join(dest, ".pachinko-state")).st_mode)
        except FileNotFoundError:
            marked = False
        if not marked:
            no("moved " + source + " to " + dest + ", which no longer holds its marker; nothing was removed. Look at " + dest + " and remove it by hand if it is the state")
    elif command == "machines":
        listing = json.loads(sys.stdin.read())
        if not isinstance(listing, list):
            no("the listing is not a JSON list")
        names = []
        for row in listing:
            name = row.get("name") if isinstance(row, dict) else None
            if not isinstance(name, str) or not name or "\n" in name:
                no("the listing holds a row without a name: " + repr(row)[:200])
            names.append(name)
        if args[:1] == ["--has"]:
            print("listed" if args[1] in names else "unlisted")
        else:
            for name in names:
                print(name)
    elif command == "worktree":
        path, wanted = args[0], args[1]
        id_path = os.path.join(path, ".tools", "smolvm-state-id")
        try:
            os.lstat(path)
        except FileNotFoundError:
            print("gone")
            sys.exit(0)
        try:
            tools = os.lstat(os.path.join(path, ".tools"))
        except FileNotFoundError:
            print("gone")
            sys.exit(0)
        if stat.S_ISLNK(tools.st_mode) or not stat.S_ISDIR(tools.st_mode):
            no(os.path.join(path, ".tools") + " is a symbolic link or not a directory, so its id says nothing about that worktree")
        try:
            st = os.lstat(id_path)
        except FileNotFoundError:
            print("gone")
            sys.exit(0)
        if stat.S_ISLNK(st.st_mode) or not stat.S_ISREG(st.st_mode):
            no(id_path + " is not a regular file")
        with open(id_path, encoding="ascii") as handle:
            text = handle.read()
        if not re.fullmatch(r"[0-9a-f]{32}\n", text):
            no(id_path + " does not hold one line of 32 lowercase hex digits")
        print("live" if text.strip() == wanted else "gone")
    else:
        sys.stderr.write("unknown helper command " + command + "\n")
        sys.exit(2)
except OSError as error:
    sys.stderr.write(str(error) + "\n")
    sys.exit(2)
'

h() {
    "$py" -I -B -c "$helper" "$@"
}

# No ACL entry may allow anything: the owner and the mode bits must say all who can
# write. macOS sets deny-only entries (group:everyone deny delete) on the home
# directory, Library and Library/Caches, which grant nothing and are accepted.
no_allow_acl() {
    acl=$(/bin/ls -lde -- "$1") || refuse "cannot read the ACL of $1"
    case $acl in
        *' allow '*) refuse "$1 has an ACL entry that allows access beyond its mode bits; remove it (chmod -N $1)" ;;
    esac
}

check_dir() { # check_dir <path> [exact octal mode]
    why=$(h dir "$@" 2>&1) || refuse "$why; it must be a directory of yours that no one else can write"
    no_allow_acl "$1"
}

# lock <path> <descriptor>: take the lock, refusing at once when it is busy.
lock() {
    lock_path=$1 lock_fd=$2
    lock_state=$(h lexists "$lock_path") || refuse "cannot check $lock_path"
    if [ "$lock_state" = absent ]; then
        h mklock "$lock_path" || refuse "cannot make $lock_path"
    fi
    why=$(h file "$lock_path" 2>&1) || refuse "$why; the lock must be a regular file of yours"
    case $lock_fd in
        8) exec 8<"$lock_path" ;;
        9) exec 9<"$lock_path" ;;
    esac
    rc=0
    /usr/bin/lockf -s -t 0 "$lock_fd" || rc=$?
    if [ "$rc" -ne 0 ]; then
        # Close this process's own descriptor first, so lsof names only the others.
        case $lock_fd in
            8) exec 8<&- ;;
            9) exec 9<&- ;;
        esac
        pids=$(/usr/sbin/lsof -t -- "$lock_path" 2>/dev/null | /usr/bin/paste -s -d ',' -) || pids=''
        holders=''
        [ -z "$pids" ] || holders=$(/bin/ps -o pid=,comm= -p "$pids" 2>/dev/null | /usr/bin/sed 's/^ *\([0-9]*\) \(.*\)$/pid \1 (\2)/' | /usr/bin/paste -s -d ',' -) || holders=''
        if [ "$rc" -eq 75 ]; then
            refuse "$lock_path is held by another smolvm command for this worktree (${holders:-a process lsof cannot name}); wait for it to end, or end it, then run again"
        fi
        refuse "cannot lock $lock_path (lockf exit $rc)"
    fi
    why=$(h fd "$lock_path" "$lock_fd" 2>&1) || refuse "$why"
    no_allow_acl "$lock_path"
}

install_check() {
    [ -d .tools ] && [ ! -L .tools ] ||
        refuse ".tools is missing or a symbolic link; run just install-smolvm (remove a link itself, not its target)"
    lock "$install_lock" 8
    /bin/sh scripts/install-smolvm.sh --check >&2 ||
        refuse "smolvm is not installed as $pin_file pins it; run just install-smolvm"
    pin=$(/usr/bin/sed -n 's/^sha256=//p' "$pin_file")
}

caller_env() {
    var_names=$(/usr/bin/env | /usr/bin/sed -n 's/^\([A-Za-z_][A-Za-z0-9_]*\)=.*/\1/p')
    for var_name in $var_names; do
        case $var_name in
            SMOLVM_* | DYLD_* | XDG_CACHE_HOME | XDG_DATA_HOME | XDG_CONFIG_HOME | DOCKER_CONFIG)
                refuse "$var_name is set; it would move smolvm's state or change what it loads, which this launcher fixes (decision 0022); unset it, then run again"
                ;;
        esac
    done
}

caller_args() {
    for arg in "$@"; do
        case $arg in
            --) return 0 ;;
            --state-dir | --state-dir=*)
                refuse "--state-dir would move smolvm's state, which this launcher keeps where decision 0022 says"
                ;;
        esac
    done
}

# Sets home, cache (the parent of every state directory), and checks each parent
# from the home directory down, making an absent one. The two this launcher owns,
# Library/Caches/pachinko and its smolvm/, must have mode 0700 exactly, since
# smolvm opens the state directory to 0755 and they alone keep others out of it.
parents() {
    home=$(h home) || refuse "cannot read your home directory from the user database"
    [ "${HOME-}" = "$home" ] ||
        refuse "HOME is '${HOME-}', not $home, your home directory in the user database; this launcher keeps state under the latter"
    check_dir "$home"
    for p in Library Library/Caches Library/Caches/pachinko Library/Caches/pachinko/smolvm; do
        state=$(h lexists "$home/$p") || refuse "cannot check $home/$p"
        if [ "$state" = absent ]; then
            /bin/mkdir -m 0700 -- "$home/$p" 2>/dev/null || [ -d "$home/$p" ] ||
                refuse "cannot make $home/$p"
        fi
        case $p in
            Library/Caches/pachinko*) check_dir "$home/$p" 0700 ;;
            *) check_dir "$home/$p" ;;
        esac
    done
    cache=$home/Library/Caches/pachinko/smolvm
}

# Sets state_dir, state_link, marker, link_hash from the worktree at $1 (a physical
# path); <name>-<hash> as decision 0022 forms it.
names_for() {
    out=$(h ident "$1") || refuse "cannot form the state's name for $1"
    base=$(/usr/bin/printf '%s\n' "$out" | /usr/bin/sed -n 1p)
    link_hash=$(/usr/bin/printf '%s\n' "$out" | /usr/bin/sed -n 2p)
    state_dir=$cache/$base
    state_link=/tmp/pachinko-smolvm-$link_hash
    marker=$state_dir/.pachinko-state
    [ "${#state_link}" -le "$link_limit" ] ||
        refuse "the state link $state_link is longer than $link_limit bytes, smolvm's limit for a state root"
}

read_marker() { # sets m_path, m_id, m_pin from $marker
    why=$(h file "$marker" 2>&1) ||
        refuse "$why; the state directory $state_dir has no valid marker. If its worktree is gone, run just smolvm-purge --orphan ${state_dir##*/}"
    fields=$(h marker "$marker" 2>&1) ||
        refuse "$fields; the marker $marker must hold path= (absolute), id= (32 lowercase hex) and pin= (64 lowercase hex), once each and nothing else"
    m_id=$(/usr/bin/printf '%s\n' "$fields" | /usr/bin/sed -n 1p)
    m_pin=$(/usr/bin/printf '%s\n' "$fields" | /usr/bin/sed -n 2p)
    m_path=$(/usr/bin/printf '%s\n' "$fields" | /usr/bin/sed -n 3p)
}

read_id() { # sets wt_id from the id file, which must be valid
    why=$(h file "$id_file" 2>&1) || refuse "$why; $id_file must be a regular file"
    wt_id=$(h id "$id_file" 2>&1) || refuse "$wt_id"
}

state_inner() { # tmp/ and docker-config/, on every entry; a missing one is made again
    for sub in tmp docker-config; do
        sub_state=$(h lexists "$state_dir/$sub") || refuse "cannot check $state_dir/$sub"
        if [ "$sub_state" = absent ]; then
            /bin/mkdir -m 0700 -- "$state_dir/$sub" || refuse "cannot make $state_dir/$sub"
            say "made $state_dir/$sub again, which was missing"
        fi
    done
    check_dir "$state_dir/tmp" 0700
    check_dir "$state_dir/docker-config" 0700
    why=$(h empty "$state_dir/docker-config" 2>&1) ||
        refuse "$why; smolvm's registry configuration must stay empty (no config.json, no link)"
}

the_link() { # make the state link if absent, then check it without following it
    state=$(h lexists "$state_link") || refuse "cannot check $state_link"
    if [ "$state" = absent ]; then
        h symlink "$state_dir" "$state_link" 2>/dev/null ||
            refuse "cannot make the state link $state_link"
    fi
    why=$(h link "$state_link" "$state_dir" 2>&1) ||
        refuse "$why; $state_link must be your symbolic link to $state_dir. Remove the entry itself (not what it points to), then run again"
}

no_links() {
    found=$(/usr/bin/find "$state_dir" -type l -print) ||
        refuse "cannot walk $state_dir to check it for symbolic links; make every directory under it readable"
    [ -z "$found" ] ||
        refuse "the state directory holds a symbolic link, which smolvm would write through: $(/usr/bin/printf '%s' "$found" | /usr/bin/tr '\n' ' '); remove the link itself (not its target), then run again"
}

# This worktree's state, under both locks. With `create`, absent state is made.
own_state() {
    want=$1
    parents
    names_for "$root"
    lock "$state_dir.lock" 9
    parents
    state=$(h lexists "$state_dir") || refuse "cannot check $state_dir"
    if [ "$state" = present ]; then
        # Wholly empty: a creation interrupted before its marker. Nothing is in it
        # to adopt or to lose, so it is removed (and, for run, made again below).
        check_dir "$state_dir"
        if h interrupted "$state_dir" 2>/dev/null; then
            /bin/rmdir -- "$state_dir" || refuse "cannot remove the empty state directory $state_dir"
            say "removed the state directory $state_dir, which an interrupted creation left empty"
            state=absent
        fi
    fi
    if [ "$state" = absent ]; then
        if [ "$want" != create ]; then
            return 1
        fi
        id_state=$(h lexists "$id_file") || refuse "cannot check $id_file"
        if [ "$id_state" = absent ]; then
            new_id=$(/usr/bin/od -An -N16 -tx1 /dev/urandom | /usr/bin/tr -d ' \n') ||
                refuse "cannot read /dev/urandom for the worktree's id"
            h write-new "$id_file" "$new_id
" || refuse "cannot write $id_file"
        fi
        read_id
        # The marker first, at once, so an interruption leaves either an empty
        # directory (removed above on the next entry) or a marked one whose missing
        # tmp/ or docker-config/ state_inner makes again.
        /bin/mkdir -m 0700 -- "$state_dir" || refuse "cannot make $state_dir"
        h write-new "$marker" "path=$root
id=$wt_id
pin=$pin
" || refuse "cannot write $marker"
        /bin/mkdir -m 0700 -- "$state_dir/tmp" "$state_dir/docker-config" ||
            refuse "cannot make the directories inside $state_dir"
        say "made this worktree's state: $state_dir"
    fi
    # smolvm sets its data root to 0755 on first use, so the state directory is held
    # to the parents' rule (no write by group or others); its parent is 0700, so no
    # one else can reach it. Its device and inode are recorded for the purge.
    check_dir "$state_dir"
    state_devino=$(h devino "$state_dir") || refuse "cannot read the device and inode of $state_dir"
    read_marker
    [ "$m_path" = "$root" ] ||
        refuse "$state_dir belongs to $m_path, not this worktree ($root)"
    read_id
    [ "$m_id" = "$wt_id" ] ||
        refuse "$state_dir was made for another worktree at this path (its id differs from $id_file); if that worktree is gone, run just smolvm-purge --orphan ${state_dir##*/}"
    state_inner
    the_link
    no_links
    return 0
}

# inventory <link>: smolvm's own listing of the machines in that state; sets
# machines, one name per line. Any failure is a refusal.
inventory() {
    # In the foreground, into a file in the state's own tmp/, so that the trap makes
    # this shell, and its locks, outlast the listing (a command substitution would
    # not wait for it on a signal).
    listing_file=$(/usr/bin/mktemp "$state_dir/tmp/listing.XXXXXX") ||
        refuse "cannot make a file for smolvm's listing in $state_dir/tmp"
    rc=0
    /usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="$1" TERM="${TERM:-dumb}" \
        TMPDIR="$1/tmp" DOCKER_CONFIG="$1/docker-config" SMOLVM_DATA_DIR="$1" \
        "$release" machine ls --json >"$listing_file" 8<&- 9<&- || rc=$?
    listing=$(/bin/cat -- "$listing_file") || rc=1
    /bin/rm -f -- "${listing_file:?}"
    [ "$rc" -eq 0 ] ||
        refuse "smolvm could not list the machines in $1, so nothing was removed"
    machines=$(/usr/bin/printf '%s' "$listing" | h machines) ||
        refuse "smolvm's listing of the machines in $1 cannot be read, so nothing was removed"
}

run_mode() {
    trap ':' HUP INT TERM
    install_check
    caller_env
    caller_args "$@"
    own_state create
    if [ "$m_pin" != "$pin" ]; then
        case ${1-}:${2-} in
            machine:ls | machine:stop | machine:delete) ;;
            *) refuse "$state_dir was made under another smolvm pin; only machine ls, stop and delete run until it is purged: delete its machines, then run just smolvm-purge" ;;
        esac
    fi
    /usr/bin/printf '%s\n' "$sentinel" >&2 || exit 1
    rc=0
    /usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="$state_link" TERM="${TERM:-dumb}" TMPDIR="$state_link/tmp" DOCKER_CONFIG="$state_link/docker-config" SMOLVM_DATA_DIR="$state_link" "$release" "$@" 8<&- 9<&- || rc=$?
    exit "$rc"
}

purge_mode() {
    trap ':' HUP INT TERM
    install_check
    caller_env
    if ! own_state keep; then
        found=$(h lexists "$state_link") || refuse "cannot check $state_link"
        if [ "$found" = present ]; then
            why=$(h link "$state_link" "$state_dir" 2>&1) ||
                refuse "$why; leaving $state_link as it is"
            /bin/rm -f -- "${state_link:?}" || refuse "cannot remove $state_link"
            say "removed the state link $state_link, whose state was already gone"
        fi
        say "no state for this worktree ($state_dir); nothing to purge"
        exit 0
    fi
    remove_state "just smolvm machine delete -f --name"
}

# remove_state <how to delete a machine>: the inventory, then the removal. The
# state directory is renamed aside, on its own filesystem, and checked to be the one
# validated before anything is removed; only the renamed directory is removed.
remove_state() {
    inventory "$state_link"
    if [ -n "$machines" ]; then
        old_ifs=$IFS
        IFS=$nl
        set -f
        for m in $machines; do
            say "machine left in $state_dir: $m; delete it with: $1 $m"
        done
        set +f
        IFS=$old_ifs
        refuse "$state_dir still holds machines (above), so nothing was removed"
    fi
    why=$(h one-fs "$state_dir" 2>&1) || refuse "$why; nothing was removed"
    purging=$cache/.purging-$link_hash-$$
    why=$(h aside "$state_dir" "$purging" "$state_devino" 2>&1) || refuse "$why"
    /bin/rm -f -- "${state_link:?}" ||
        refuse "cannot remove $state_link; the state directory is now $purging, which nothing uses: remove it by hand"
    /bin/rm -rf -- "${purging:?}" ||
        refuse "cannot remove $purging, the state directory renamed for removal, which nothing uses: remove it by hand"
    say "removed the state link $state_link"
    say "removed the state directory $state_dir"
    say "kept the lock entry $state_dir.lock"
}

orphan_mode() {
    trap ':' HUP INT TERM
    name=$1
    shift
    delete=''
    if [ $# -gt 0 ]; then
        [ "$1" = --delete ] && [ $# -eq 2 ] && [ -n "$2" ] ||
            refuse "usage: just smolvm-purge --orphan <name> [--delete <machine>]"
        case $2 in
            -*) refuse "'$2' starts with -, which smolvm would read as an option, not a machine's name" ;;
        esac
        delete=$2
    fi
    case $name in
        *"$nl"*) refuse "the name given to --orphan holds a newline; it is not a state directory's name" ;;
    esac
    /usr/bin/printf '%s\n' "$name" | /usr/bin/grep -Eqx '[a-z0-9-]+-[0-9a-f]{8}' ||
        refuse "'$name' is not a state directory's name (<worktree>-<8 hex>), as listed under ~/Library/Caches/pachinko/smolvm/"
    install_check
    caller_env
    parents
    state_dir=$cache/$name
    marker=$state_dir/.pachinko-state
    orphan_checks
    lock "$state_dir.lock" 9
    parents
    orphan_checks
    life=$(h worktree "$m_path" "$m_id" 2>&1) ||
        refuse "cannot tell whether $m_path, the worktree $state_dir names, still exists ($life); make it searchable, or run just smolvm-purge from it"
    [ "$life" = gone ] ||
        refuse "$m_path still exists and holds this state's id; run just smolvm-purge from that worktree"
    state_inner
    the_link
    no_links
    if [ -n "$delete" ]; then
        inventory "$state_link"
        listed=$(/usr/bin/printf '%s' "$listing" | h machines --has "$delete") ||
            refuse "smolvm's listing of the machines in $state_link cannot be read, so nothing was deleted"
        [ "$listed" = listed ] ||
            refuse "'$delete' is not a machine the listing of $state_dir holds"
        rc=0
        /usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="$state_link" TERM="${TERM:-dumb}" TMPDIR="$state_link/tmp" DOCKER_CONFIG="$state_link/docker-config" SMOLVM_DATA_DIR="$state_link" "$release" machine delete -f --name "$delete" 8<&- 9<&- || rc=$?
        exit "$rc"
    fi
    remove_state "just smolvm-purge --orphan $name --delete"
}

# The orphan's own identity, run once before its lock (read-only, so a name that
# fails makes no lock entry) and again under it, where the device and inode it
# records are the ones the removal checks.
orphan_checks() {
    check_dir "$state_dir"
    state_devino=$(h devino "$state_dir") || refuse "cannot read the device and inode of $state_dir"
    read_marker
    case $m_path in
        /*) ;;
        *) refuse "the marker $marker records a path that is not absolute" ;;
    esac
    out=$(h ident "$m_path") || refuse "cannot form the state's name for $m_path"
    [ "$(/usr/bin/printf '%s\n' "$out" | /usr/bin/sed -n 1p)" = "${state_dir##*/}" ] ||
        refuse "$state_dir is not named for the worktree its marker records ($m_path)"
    link_hash=$(/usr/bin/printf '%s\n' "$out" | /usr/bin/sed -n 2p)
    state_link=/tmp/pachinko-smolvm-$link_hash
}

[ $# -gt 0 ] || refuse "usage: smolvm.sh run <smolvm arguments> | purge [--orphan <name> [--delete <machine>]]"
mode=$1
shift
case $mode in
    run) run_mode "$@" ;;
    purge)
        if [ $# -eq 0 ]; then
            purge_mode
        elif [ "$1" = --orphan ] && [ $# -ge 2 ]; then
            shift
            orphan_mode "$@"
        else
            refuse "usage: just smolvm-purge [--orphan <name> [--delete <machine>]]"
        fi
        ;;
    *) refuse "unknown mode '$mode'; use run or purge" ;;
esac
