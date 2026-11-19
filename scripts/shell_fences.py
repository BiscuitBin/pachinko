"""Lint the shell fences of the ticket Markdown files it is given.

The `shell-fences` hook in .pre-commit-config.yaml runs it, inside the
shellcheck-py hook's environment, so the shellcheck it finds on PATH is the one
that hook pins (T37, decision C). Standard library only, and no particular
Python version: prek builds that environment with its own interpreter.

A file whose frontmatter says `status: done` is skipped, because a closed
ticket's commands are a record of what ran (T37, decision B). Every other file
is linted, including one with no frontmatter or no status, such as
CONVENTIONS.md and README.md. A fence tagged `sh` is checked as POSIX sh and one
tagged `bash` as bash, each with `-o check-extra-masked-returns`. The script
fails closed: an untagged fence, a shell fence under another tag, a fence it
cannot close and a fence inside a block quote are errors, never silent passes.
"""

import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

DIALECTS = {"sh": "sh", "bash": "bash"}
# Tags that name a shell but not a dialect this hook checks. A fence under one
# of them is shell the hook would otherwise never read, so it is an error.
SHELL_TAGS = {
    "ash", "console", "dash", "fish", "ksh", "mksh", "posix", "sh-session",
    "shell", "shell-session", "shellscript", "shellsession", "zsh",
}
OPENER = re.compile(r"^(?P<indent>[ \t]*)(?P<fence>`{3,}|~{3,})(?P<info>.*)$")
QUOTED_OPENER = re.compile(r"^[ \t]*>[ \t>]*(`{3,}|~{3,})")
STATUS = re.compile(r"^status:[ \t]*(?P<value>.*?)[ \t]*$")
GCC = re.compile(r"^(?P<line>\d+):(?P<col>\d+): (?P<rest>.*)$")


def is_done(lines):
    """True only when the frontmatter holds exactly one `status:` and it is `done`."""
    if not lines or lines[0].rstrip() != "---":
        return False
    statuses = []
    for line in lines[1:]:
        if line.rstrip() in ("---", "..."):
            return len(statuses) == 1 and statuses[0].strip("\"'") == "done"
        match = STATUS.match(line)
        if match:
            statuses.append(match.group("value"))
    return False


def tag_of(info):
    """The fence's tag: the info string's first word, lowercased, cut at a non-word character."""
    words = info.split()
    if not words:
        return ""
    return re.match(r"[\w-]*", words[0].lower()).group()


def fences(lines):
    """Yield (opener line number, tag, body, indent widths) or (line number, None, error, None)."""
    i = 0
    while i < len(lines):
        line = lines[i]
        if QUOTED_OPENER.match(line):
            yield i + 1, None, "fence inside a block quote, which this hook does not read", None
            i += 1
            continue
        match = OPENER.match(line)
        if not match or (match.group("fence")[0] == "`" and "`" in match.group("info")):
            i += 1
            continue
        indent, fence = match.group("indent"), match.group("fence")
        closer = re.compile(r"^[ \t]*" + re.escape(fence[0]) + "{" + str(len(fence)) + r",}[ \t]*$")
        body, widths = [], []
        j = i + 1
        while j < len(lines) and not closer.match(lines[j]):
            text = lines[j]
            if text.startswith(indent):
                body.append(text[len(indent):])
                widths.append(len(indent))
            else:
                body.append(text)
                widths.append(0)
            j += 1
        if j == len(lines):
            yield i + 1, None, "fence opened here is never closed", None
            return
        yield i + 1, tag_of(match.group("info")), body, widths
        i = j + 1


def run_shellcheck(shellcheck, workdir, dialect, body):
    """Return (exit status, stdout, stderr) of shellcheck over one fence's body."""
    path = Path(workdir) / "fence.sh"
    path.write_text("\n".join(body) + "\n", encoding="utf-8")
    env = {k: v for k, v in os.environ.items() if k != "SHELLCHECK_OPTS"}
    command = [
        shellcheck, "--norc", "-s", dialect, "-o", "check-extra-masked-returns",
        "-f", "gcc", str(path),
    ]
    done = subprocess.run(command, capture_output=True, text=True, env=env, check=False)
    prefix = str(path) + ":"
    out = "\n".join(
        line[len(prefix):] if line.startswith(prefix) else line
        for line in done.stdout.splitlines()
    )
    return done.returncode, out, done.stderr


def check_file(name, shellcheck, workdir):
    """Lint one file. Return (fences checked, problems, summary line)."""
    try:
        text = Path(name).read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        return 0, [f"{name}: cannot read: {error}"], f"{name}: unreadable"
    lines = [line.rstrip("\r") for line in text.split("\n")]
    if is_done(lines):
        return 0, [], f"{name}: skipped: status done"
    checked, problems, skipped = 0, [], {}
    for start, tag, body, widths in fences(lines):
        if tag is None:
            problems.append(f"{name}:{start}: {body}")
        elif tag == "":
            problems.append(f"{name}:{start}: untagged fence; tag it sh, bash or what it holds")
        elif tag in SHELL_TAGS:
            problems.append(f"{name}:{start}: fence tagged {tag}; retag it sh or bash")
        elif tag not in DIALECTS:
            skipped[tag] = skipped.get(tag, 0) + 1
        else:
            checked += 1
            status, out, err = run_shellcheck(shellcheck, workdir, DIALECTS[tag], body)
            if status == 0:
                continue
            if status != 1:
                problems.append(f"{name}:{start}: shellcheck exited {status}: {err.strip()}")
                continue
            mapped = []
            for line in out.splitlines():
                match = GCC.match(line)
                if match:
                    k = int(match.group("line"))
                    col = int(match.group("col")) + (widths[k - 1] if k <= len(widths) else 0)
                    mapped.append(f"{name}:{start + k}:{col}: {match.group('rest')}")
            if not mapped:
                mapped.append(f"{name}:{start}: shellcheck failed this fence: {out or err}".rstrip())
            problems.extend(mapped)
    if checked:
        summary = f"{name}: {checked} checked"
    else:
        summary = f"{name}: no sh or bash fence; checked nothing"
    if skipped:
        summary += "; other tags not checked: " + ", ".join(
            f"{tag} {count}" for tag, count in sorted(skipped.items()))
    return checked, problems, summary


def plural(count, noun):
    return f"{count} {noun}" if count == 1 else f"{count} {noun}s"


def main(argv):
    shellcheck = shutil.which("shellcheck")
    if shellcheck is None:
        print("shell_fences: shellcheck is not on PATH; run this through the shell-fences hook")
        return 2
    total, linted, done, problems = 0, 0, 0, []
    with tempfile.TemporaryDirectory() as workdir:
        for name in argv:
            checked, found, summary = check_file(name, shellcheck, workdir)
            print(summary)
            if summary.endswith("skipped: status done"):
                done += 1
            else:
                linted += 1
            total += checked
            problems.extend(found)
    for problem in problems:
        print(problem)
    print(f"checked {plural(total, 'fence')} in {plural(linted, 'file')}; "
          f"skipped {done} with status done")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
