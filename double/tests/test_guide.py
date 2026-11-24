"""The reader's guide, double/README.md, runs as written.

Its walkthrough of decision 0020's four lease endings is a doctest session.
The test parses the whole page as one doctest, so later examples see the
setting the earlier ones define, and runs it with a runner that records each
example it starts. It fails with doctest's own report of each example whose
output differs. It runs inside an ordinary test method, so test_coverage sees
it collected and passed like any other test.

An example that is not run cannot fail, so the test also checks that each one
the page shows ran. A `+SKIP` directive collects an example without running
it, so every collected example must have started. A lost `>>>` prompt turns
an example into prose that doctest never collects, so every `pycon` block
must hold an example that started. And each of the walkthrough's four
scenario sections, Scenarios A to D, must hold such a block.
"""

import doctest
import re
import unittest
from collections.abc import Callable
from pathlib import Path

GUIDE = Path(__file__).resolve().parents[1] / "README.md"

SCENARIO_HEADING = re.compile(r"^### Scenario (?P<letter>[A-Z]):")

HEADING = re.compile(r"^#{1,3} ")

SCENARIOS = ("A", "B", "C", "D")


class RecordingRunner(doctest.DocTestRunner):
    """A doctest runner that records the line of each example it starts."""

    def __init__(self) -> None:
        super().__init__(verbose=False)
        self.started: list[int] = []

    def report_start(self, out: Callable[[str], object], test: doctest.DocTest, example: doctest.Example) -> None:
        self.started.append(example.lineno)


def pycon_blocks(lines: list[str]) -> list[range]:
    """The 0-based lines of each fenced `pycon` block, fence to fence."""
    blocks = []
    opened = None
    for index, text in enumerate(lines):
        if opened is None and text == "```pycon":
            opened = index
        elif opened is not None and text == "```":
            blocks.append(range(opened, index + 1))
            opened = None
    return blocks


def scenario_sections(lines: list[str]) -> dict[str, range]:
    """The 0-based lines of each `### Scenario X:` section, up to the next
    heading of the same or a higher level, or the end of the page."""
    sections = {}
    for start, text in enumerate(lines):
        found = SCENARIO_HEADING.match(text)
        if found is not None:
            following = (index for index in range(start + 1, len(lines)) if HEADING.match(lines[index]))
            sections[found["letter"]] = range(start, next(following, len(lines)))
    return sections


class GuideTest(unittest.TestCase):
    def test_the_guides_walkthrough_runs_as_written(self) -> None:
        text = GUIDE.read_text(encoding="utf-8")
        test = doctest.DocTestParser().get_doctest(text, {"__name__": "__main__"}, GUIDE.name, str(GUIDE), 0)
        reports: list[str] = []
        runner = RecordingRunner()
        failed, _ = runner.run(test, out=reports.append, clear_globs=True)
        self.assertEqual(failed, 0, "".join(reports))
        self.assertGreater(len(test.examples), 0, "the guide holds no doctest examples")
        self.assertEqual(len(runner.started), len(test.examples), "every collected example runs")
        lines = text.splitlines()
        blocks = pycon_blocks(lines)
        for block in blocks:
            with self.subTest(block=block.start + 1):
                ran = [line for line in runner.started if line in block]
                self.assertGreater(len(ran), 0, "a pycon block holds no example that ran")
        sections = scenario_sections(lines)
        self.assertEqual(sorted(sections), list(SCENARIOS), "the walkthrough's scenario headings")
        for letter in SCENARIOS:
            with self.subTest(scenario=letter):
                held = [block for block in blocks if block.start in sections[letter]]
                self.assertGreater(len(held), 0, f"Scenario {letter} holds no pycon block")
