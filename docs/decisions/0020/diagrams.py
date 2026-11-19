"""Draw the SVG diagrams decision 0020 embeds, into this script's own folder.

Run it through `just diagrams`, which finds every docs/decisions/*/diagrams.py.
The script owns each diagram's layout and words: edit it and rerun the recipe,
never the SVG. With `--check` it writes nothing and fails, naming the file, on
an SVG here, in any letter case, that differs from what it draws, is missing,
or is drawn by no diagram; `just check-docs` runs that. Standard library only.
Every SVG carries its own light and dark colours, chosen by
`prefers-color-scheme`, and an opaque background, so it stays legible when the
page's theme and the reader's system theme differ.

Each diagram restates rules from docs/specs/pachinko.allium; where a drawing and
the specification differ, the specification wins.
"""

import math
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[2]

LIGHT = {
    "bg": "#ffffff", "fg": "#1f2328", "muted": "#59636e", "line": "#d1d9e0",
    "box": "#f6f8fa", "info": "#ddf4ff", "ok": "#dafbe1", "warn": "#fff8c5",
    "bad": "#ffebe9", "accent": "#0969da",
}
DARK = {
    "bg": "#0d1117", "fg": "#e6edf3", "muted": "#9198a1", "line": "#3d444d",
    "box": "#151b23", "info": "#0c2d6b", "ok": "#0f3d1d", "warn": "#3b2e00",
    "bad": "#5d0f12", "accent": "#4493f8",
}
FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif"


def esc(s):
    return str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")


def style():
    def tokens(palette):
        return " ".join(f"--{k}: {v};" for k, v in palette.items())

    return "\n".join([
        "<style>",
        f"svg {{ {tokens(LIGHT)} }}",
        f"@media (prefers-color-scheme: dark) {{ svg {{ {tokens(DARK)} }} }}",
        f"text {{ font-family: {FONT}; fill: var(--fg); }}",
        "text.muted { fill: var(--muted); }",
        ".bg { fill: var(--bg); }",
        ".lane { stroke: var(--line); stroke-width: 1; }",
        ".edge { fill: none; stroke: var(--fg); stroke-width: 1.4; }",
        ".edge.accent { stroke: var(--accent); }",
        ".edge.dash { stroke-dasharray: 5 4; }",
        ".head { fill: var(--fg); }",
        ".head.accent { fill: var(--accent); }",
        ".frame { fill: none; stroke: var(--fg); stroke-width: 1.4; }",
        ".box { stroke: var(--line); stroke-width: 1.2; }",
        ".box.dash { stroke-dasharray: 5 4; }",
        ".box.plain { fill: var(--box); } .box.info { fill: var(--info); } .box.ok { fill: var(--ok); }",
        ".box.warn { fill: var(--warn); } .box.bad { fill: var(--bad); } .box.none { fill: var(--bg); }",
        "</style>",
    ])


def text(x, y, s, size=13, muted=False, anchor="start", bold=False, middle=False):
    lines = str(s).split("\n")
    step = size * 1.3
    y0 = y - (len(lines) - 1) * step / 2 if middle else y
    attrs = f'x="{x:g}" y="{y0:g}" font-size="{size:g}" text-anchor="{anchor}" dominant-baseline="middle"'
    if muted:
        attrs += ' class="muted"'
    if bold:
        attrs += ' font-weight="600"'
    spans = "".join(
        f'<tspan x="{x:g}" dy="{step if i else 0:g}">{esc(line)}</tspan>' for i, line in enumerate(lines)
    )
    return f"<text {attrs}>{spans}</text>"


def box(x, y, w, h, label="", fill="plain", size=13, bold=False, dash=False):
    cls = f"box {fill}" + (" dash" if dash else "")
    out = f'<rect x="{x:g}" y="{y:g}" width="{w:g}" height="{h:g}" rx="6" class="{cls}"/>'
    if label:
        out += "\n" + text(x + w / 2, y + h / 2, label, size=size, anchor="middle", bold=bold, middle=True)
    return out


def arrow(points, accent=False, dash=False):
    path = " ".join(("L" if i else "M") + f"{x:g} {y:g}" for i, (x, y) in enumerate(points))
    (x1, y1), (x2, y2) = points[-2], points[-1]
    a = math.atan2(y2 - y1, x2 - x1)
    length, half = 8, 4.5
    p1 = (x2 - length * math.cos(a) + half * math.sin(a), y2 - length * math.sin(a) - half * math.cos(a))
    p2 = (x2 - length * math.cos(a) - half * math.sin(a), y2 - length * math.sin(a) + half * math.cos(a))
    cls = "edge" + (" accent" if accent else "") + (" dash" if dash else "")
    head = "head" + (" accent" if accent else "")
    return (
        f'<path d="{path}" class="{cls}"/>\n'
        f'<polygon points="{x2:g},{y2:g} {p1[0]:.1f},{p1[1]:.1f} {p2[0]:.1f},{p2[1]:.1f}" class="{head}"/>'
    )


def lanes(names, top, right):
    out = []
    for name, y in names:
        out.append(text(10, top + y, name, size=12, muted=True))
        out.append(f'<line x1="130" y1="{top + y:g}" x2="{right:g}" y2="{top + y:g}" class="lane"/>')
    return out


def svg(width, height, title, desc, body):
    return "\n".join([
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}"'
        ' role="img" aria-labelledby="title desc">',
        f'<title id="title">{esc(title)}</title>',
        f'<desc id="desc">{esc(desc)}</desc>',
        style(),
        f'<rect width="{width}" height="{height}" rx="8" class="bg"/>',
        *body,
        "</svg>",
        "",
    ])


def defect():
    """Diagram A: open questions 10 and 9, under the rules before decision 0020."""
    b = [text(130, 14, "Open question 10: one version shed twice", bold=True)]
    b += lanes([("publisher", 40), ("consumer c1", 84), ("record k", 128), ("superseded queue", 176),
                ("dead letters", 220)], 0, 750)
    b.append(arrow([(200, 90), (200, 118)]))
    b.append(text(208, 104, "pull", size=12))
    b.append(box(154, 141, 100, 24, "rank 1, held", fill="info", size=12))
    b.append(arrow([(360, 46), (360, 118)]))
    b.append(text(368, 58, "publish rank 2", size=12))
    b.append(box(318, 141, 84, 24, "rank 2", size=12))
    b.append(arrow([(402, 155), (440, 170)], accent=True))
    b.append(box(440, 164, 64, 24, "rank 1", fill="bad", size=12))
    b.append(text(330, 199, "Supersede sheds the version c1 holds", size=11, muted=True))
    b.append(arrow([(580, 90), (580, 118)]))
    b.append(text(588, 104, "nack k", size=12))
    b.append(arrow([(580, 132), (580, 208)], accent=True))
    b.append(box(548, 208, 64, 24, "rank 1", fill="bad", size=12))
    b.append(text(590, 199, "nack dead-letters it too", size=11, muted=True))
    b.append(text(130, 254, "Rank 1 ends in both shed queues.", size=12, bold=True))

    top = 280
    b.append(text(130, top + 14, "Open question 9: one lease handed k twice", bold=True))
    b += lanes([("publisher", 40), ("consumer c1", 84), ("record k", 128)], top, 750)
    b.append(arrow([(190, top + 90), (190, top + 118)]))
    b.append(text(198, top + 104, "pull, lease L", size=12))
    b.append(box(144, top + 141, 100, 24, "rank 1, held", fill="info", size=12))
    b.append(arrow([(320, top + 90), (320, top + 118)]))
    b.append(text(328, top + 104, "ack k", size=12))
    b.append(box(280, top + 141, 80, 24, "removed", fill="none", size=12, dash=True))
    b.append(arrow([(450, top + 46), (450, top + 118)]))
    b.append(text(458, top + 58, "publish rank 2", size=12))
    b.append(box(394, top + 141, 112, 24, "rank 2, queued", size=12))
    b.append(arrow([(620, top + 90), (620, top + 118)]))
    b.append(text(628, top + 104, "pull, lease L", size=12))
    b.append(box(570, top + 141, 100, 24, "rank 2, held", fill="bad", size=12))
    b.append(text(130, top + 190, "Lease L is handed k a second time.", size=12, bold=True))
    return svg(
        760, top + 210,
        "The defect before decision 0020",
        "Two timelines under the rules before decision 0020. Open question 10: consumer c1 pulls k at "
        "rank 1; a publish of rank 2 makes Supersede shed rank 1, which c1 still holds, into the superseded "
        "queue; c1's nack then makes rank 1 a dead letter too. Open question 9: under lease L, c1 pulls k at "
        "rank 1 and acks it, which removes the record; a publish of rank 2 creates a new, queued record; "
        "c1 pulls again under L and is handed k a second time.",
        b,
    )


def held_record():
    """Diagram B: a held record's life under decision 0020."""
    b = [box(20, 120, 90, 40, "queued", bold=True)]
    b.append('<rect x="170" y="50" width="310" height="190" rx="10" class="frame"/>')
    b.append(text(184, 68, "held, under one lease", bold=True))
    b.append(box(190, 122, 96, 36, "unsettled", fill="info"))
    b.append(box(366, 84, 90, 36, "acked", fill="ok"))
    b.append(box(366, 160, 90, 36, "nacked", fill="bad"))
    b.append(arrow([(286, 132), (366, 104)]))
    b.append(text(316, 108, "ack", size=12))
    b.append(arrow([(286, 148), (366, 176)]))
    b.append(text(312, 174, "nack", size=12))
    b.append(text(184, 214, "The first mark stands: a repeat only extends\nthe lease, the other is refused (MarkConflict).",
                  size=11, muted=True))
    b.append(arrow([(110, 140), (190, 140)]))
    b.append(text(130, 128, "pull", size=12))
    b.append(arrow([(325, 22), (325, 50)], accent=True, dash=True))
    waits = (
        "A publish above the stored rank waits behind the held version, and a\n"
        "still higher one replaces only the waiting one. Nothing held changes."
    )
    b.append(text(335, 16, waits, size=11, muted=True))
    b.append(box(560, 120, 110, 40, "lease end", fill="warn", bold=True))
    b.append(arrow([(480, 140), (560, 140)]))
    b.append(text(492, 126, "release", size=12))
    b.append(text(492, 156, "or lapse", size=12))
    b.append(box(712, 120, 84, 40, "removed", fill="none", dash=True))
    b.append(arrow([(670, 140), (712, 140)]))
    b.append(text(678, 96, "acked or nacked,\nnothing deferred", size=11, muted=True))
    b.append(arrow([(615, 160), (615, 280), (65, 280), (65, 160)]))
    b.append(text(340, 268, "unsettled, or a newer version deferred: queued again", size=12, anchor="middle"))
    return svg(
        810, 296,
        "A held record's life under decision 0020",
        "A record is queued until a pull under a lease makes it held. While held it is unsettled until an "
        "ack marks it acked or a nack marks it nacked; the first mark stands, a repeat only extends the "
        "lease, and the other mark is refused with MarkConflict. A publish above the stored rank waits "
        "behind the held version, and a still higher one replaces only the waiting version; nothing held "
        "changes. The lease ends by release or by lapse: a record that is unsettled, or has a newer version "
        "deferred behind it, is queued again; an acked or nacked record with nothing deferred is removed.",
        b,
    )


LEASE_END = [
    (
        "unsettled",
        ("held version → nowhere", "record → queued, same position", "RequeueUnsettledRecord", "info"),
        (
            "held version → superseded queue",
            "newer version → queued at\nthe held version's position",
            "SupersedeUnsettledHeldVersion",
            "warn",
        ),
    ),
    (
        "acked",
        ("held version → nowhere: consumed", "record → removed", "ConsumeAckedRecord", "ok"),
        (
            "held version → nowhere: consumed",
            "newer version → queued at\nits arrival position",
            "ConsumeAckedVersionQueueDeferred",
            "ok",
        ),
    ),
    (
        "nacked",
        ("held version → dead letters", "record → removed", "DeadLetterNackedRecord", "bad"),
        (
            "held version → dead letters",
            "newer version → queued at\nits arrival position",
            "DeadLetterNackedVersionQueueDeferred",
            "bad",
        ),
    ),
]


def lease_end():
    """Diagram E: the six rows of the specification's "Lease end" table."""
    x0, w, h, y0 = 110, 300, 92, 56
    b = [text(10, 14, "When a lease ends, by release or by lapse, each held record:", bold=True)]
    b.append(text(10, 40, "mark", size=12, muted=True))
    b.append(text(x0 + w / 2, 40, "nothing deferred", anchor="middle", bold=True))
    b.append(text(x0 + w * 1.5 + 10, 40, "a newer version deferred", anchor="middle", bold=True))
    for r, (mark, *cells) in enumerate(LEASE_END):
        y = y0 + r * (h + 8)
        b.append(text(10, y + h / 2, mark, bold=True))
        for c, (held, record, rule, fill) in enumerate(cells):
            x = x0 + c * (w + 10)
            b.append(f'<rect x="{x:g}" y="{y:g}" width="{w:g}" height="{h:g}" rx="6" class="box {fill}"/>')
            b.append(text(x + 10, y + 16, held, size=12, bold=True))
            b.append(text(x + 10, y + 36, record, size=12))
            b.append(text(x + 10, y + h - 12, rule, size=10.5, muted=True))
    rows = "; ".join(
        f"{mark} with {deferred}: {held}, {record.replace(chr(10), ' ')} ({rule})"
        for mark, *cells in LEASE_END
        for deferred, (held, record, rule, _) in zip(("nothing deferred", "a newer version deferred"), cells)
    )
    return svg(
        730, y0 + 3 * (h + 8) + 4,
        "The six outcomes when a lease ends",
        "When a lease ends, by release or by lapse, each held record goes by its mark and by whether a "
        f"newer version is deferred behind it. {rows}.",
        b,
    )


def positions():
    """Diagram C: where a key's newer version is queued when the lease ends."""
    cols = {11: 230, 12: 330, 13: 410, 14: 490}
    cw, row_h, y0 = 70, 52, 44
    b = [text(10, 30, "position", size=12, muted=True)]
    for p, x in cols.items():
        b.append(text(x + (45 if p == 11 else cw / 2), 30, str(p), size=12, muted=True, anchor="middle"))
    b12 = (12, "b", "plain", cw, False)
    c13 = (13, "c", "plain", cw, False)
    rows = [
        (
            "while the lease holds k r1",
            "",
            [(11, "k r1, held", "info", 90, False), b12, c13, (14, "k r2 deferred", "none", cw + 30, True)],
        ),
        ("lease end, k r1 unsettled", "k r1 → superseded queue", [(11, "k r2", "warn", 90, False), b12, c13]),
        ("lease end, k r1 acked or nacked", "k r1 consumed, or a dead letter", [b12, c13, (14, "k r2", "warn", cw, False)]),
    ]
    for r, (label, note, cells) in enumerate(rows):
        y = y0 + r * row_h
        b.append(text(10, y + 15, label, size=12, bold=True))
        if note:
            b.append(text(10, y + 33, note, size=11, muted=True))
        for p, cell, fill, w, dash in cells:
            b.append(box(cols[p], y, w, 30, cell, fill=fill, size=12, dash=dash))
    b.append(text(10, y0 + 3 * row_h + 8, "k r2 arrived after c, so it was given arrival position 14.",
                  size=11, muted=True))
    return svg(
        610, y0 + 3 * row_h + 22,
        "Where a key's newer version is queued when the lease ends",
        "A pocket's records by position. The lease holds k at rank 1 at position 11; b and c are queued at "
        "12 and 13; k at rank 2 then arrives, is deferred and is given arrival position 14. If k rank 1 is "
        "unsettled when the lease ends, it goes to the superseded queue and k rank 2 is queued at 11, its "
        "place. If k rank 1 was acked or nacked, it is consumed or becomes a dead letter, and k rank 2 is "
        "queued at 14, where it arrived, behind b and c.",
        b,
    )


DIAGRAMS = {
    "defect.svg": defect,
    "held-record.svg": held_record,
    "lease-end.svg": lease_end,
    "positions.svg": positions,
}


def render(name):
    """One diagram as the exact bytes its file holds; drawing and checking both use it."""
    return DIAGRAMS[name]().encode("utf-8")


def draw():
    for name in DIAGRAMS:
        path = OUT / name
        path.write_bytes(render(name))
        print(path.relative_to(ROOT))
    return 0


def check():
    """Compare every SVG in this folder with what the script draws; open nothing for writing."""
    script = Path(__file__).resolve().relative_to(ROOT)
    rerun = "run `just diagrams`"
    problems = []
    if not DIAGRAMS:
        problems.append(f"{script}: draws nothing; add a diagram to DIAGRAMS and {rerun}")
    for name in DIAGRAMS:
        path = OUT / name
        if not path.is_file():
            problems.append(f"{path.relative_to(ROOT)}: missing; {rerun}")
        elif path.read_bytes() != render(name):
            problems.append(f"{path.relative_to(ROOT)}: differs from what {script} draws; {rerun}")
    for path in sorted(OUT.iterdir()):
        if path.suffix.lower() == ".svg" and path.name not in DIAGRAMS:
            problems.append(
                f"{path.relative_to(ROOT)}: drawn by no diagram in {script}; "
                f"remove it, or add a diagram that draws it and {rerun}"
            )
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        return 1
    print(f"{script}: compared {len(DIAGRAMS)} SVGs, all match")
    return 0


def main(args):
    if not args:
        return draw()
    if args == ["--check"]:
        return check()
    print(f"usage: python {Path(__file__).name} [--check]", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
