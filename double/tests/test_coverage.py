"""The coverage tests: every scenario, rule and invariant is covered.

Three lists are parsed from their sources, and each count is asserted, so a
pattern that silently matches less fails:

- the catalog's scenario rows in docs/reference/testing.md, each covered by a
  scenario tag on the line above a test's `def` under double/tests/;
- the specification's rules in docs/specs/pachinko.allium, each covered by a
  row in its trigger's registered table that fired during the run. A rule's
  trigger is what its first `when:` names: an event, or for a clock trigger
  the `Entity.field` it waits on, such as `Lease.expires_at`, which is the
  name of that trigger's table;
- the specification's invariants, each covered by an entry in the checker's
  registry and by a positive control tagged on the line above its `def`.

A tag counts only when the test below it ran and passed: tests/__init__.py
records each passing test's method site, and a tag whose def is not in that
record fails. A tag above a decorated def therefore fails too, since a
decorated function's site is its first decorator's line.

Every test counts, tagged or not: each `def test` under double/tests/, in any
file and at any depth, must be among the tests load_tests collected, and, in
every module but this one, among those that ran and passed. This module's own
tests are still running when the check runs, so for them collection is all it
can see. A class that stops being a TestCase, or a test moved out of a
`test*.py` file, therefore fails the run instead of silently dropping out.

Nothing is pending: a scenario with no tag, a specification rule with no row,
and a registered row that never fired each fail the run.

The fired-rule, tag-run and test-def checks read the record the other
modules' tests leave, so they run only after them: tests/__init__.py loads this
module last. Run alone, it skips those three checks; run by
`just python-double` without going through load_tests, they fail.
"""

import ast
import os
import re
import sys
import unittest
from pathlib import Path

# Importing the package imports every table module, which completes the registry.
import pachinko_double
from pachinko_double import invariants, rules

TESTS = Path(__file__).resolve().parent
ROOT = TESTS.parents[1]
CATALOG = ROOT / "docs" / "reference" / "testing.md"
SPECIFICATION = ROOT / "docs" / "specs" / "pachinko.allium"

CATALOG_ROW = re.compile(r"^\| (S[0-9]{3}) \|", re.MULTILINE)
SPEC_RULE = re.compile(r"^rule (\w+) \{", re.MULTILINE)
SPEC_INVARIANT = re.compile(r"^\s*invariant (\w+) \{", re.MULTILINE)
# A rule's first `when:` names its trigger: an event, or for a clock trigger
# the `Entity.field` it waits on, which is the name of that trigger's table.
SPEC_WHEN = re.compile(r"^\s*when: (?:(?P<event>\w+)\(|\w+: (?P<clock>\w+\.\w+) <= now$)", re.MULTILINE)

SCENARIO_COUNT = 93
RULE_COUNT = 32
INVARIANT_COUNT = 35

# Any comment line that looks like a tag, however misspelt, then the one form
# a tag may take. A line the first matches and the second does not is a typo.
LOOSE_TAG = re.compile(r"^\s*#\s*(scenario|invariant\W*control)", re.IGNORECASE)
STRICT_TAG = re.compile(r"^(?P<indent>\s*)# (?P<kind>scenario|invariant-control): (?P<name>\S+)$")

ENV_SUITE = "PACHINKO_DOUBLE_SUITE"


def parse(pattern: re.Pattern[str], path: Path) -> list[str]:
    return pattern.findall(path.read_text(encoding="utf-8"))


def rule_triggers() -> dict[str, str | None]:
    """Each specification rule's trigger, None where its `when:` does not parse."""
    text = SPECIFICATION.read_text(encoding="utf-8")
    starts = list(SPEC_RULE.finditer(text))
    triggers: dict[str, str | None] = {}
    for start, end in zip(starts, [*starts[1:], None]):
        when = SPEC_WHEN.search(text, start.end(), end.start() if end else len(text))
        triggers[start[1]] = None if when is None else when["event"] or when["clock"]
    return triggers


def require_full_suite(case: unittest.TestCase) -> None:
    """Skip a check that reads the run's record unless load_tests ran; fail under the recipe."""
    package = sys.modules.get(__package__ or "")
    if not getattr(package, "FULL_SUITE_LOADED", False):
        if os.environ.get(ENV_SUITE) == "recipe":
            case.fail(
                "just python-double ran the coverage test without tests.load_tests, "
                "so the run's record cannot be trusted; run discovery with -t double"
            )
        case.skipTest("this check needs the whole suite: run just python-double")


def read_tags() -> tuple[dict[str, list[str]], list[str], list[tuple[str, tuple[str, int]]]]:
    """Every tag under double/tests/, by kind; every malformed one; and each tag's def site."""
    tags: dict[str, list[str]] = {"scenario": [], "invariant-control": []}
    malformed: list[str] = []
    sites: list[tuple[str, tuple[str, int]]] = []
    for path in sorted(TESTS.glob("*.py")):
        lines = path.read_text(encoding="utf-8").splitlines()
        for number, line in enumerate(lines, start=1):
            if not LOOSE_TAG.match(line):
                continue
            where = f"{path.relative_to(ROOT)}:{number}"
            strict = STRICT_TAG.match(line)
            following = lines[number] if number < len(lines) else ""
            if strict is None:
                malformed.append(f"{where}: {line.strip()!r} is not a well-formed tag")
            elif not following.startswith(f"{strict['indent']}def test"):
                malformed.append(f"{where}: {line.strip()!r} is not on the line above a test's def")
            else:
                tags[strict["kind"]].append(strict["name"])
                sites.append((f"{where}: {strict['name']}", (str(path), number + 1)))
    return tags, malformed, sites


def defined_tests() -> list[tuple[str, tuple[str, int]]]:
    """Every `def test` under double/tests/, at any depth, with its def site."""
    sites: list[tuple[str, tuple[str, int]]] = []
    for path in sorted(TESTS.glob("*.py")):
        for node in ast.walk(ast.parse(path.read_text(encoding="utf-8"), str(path))):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name.startswith("test"):
                sites.append((f"{path.relative_to(ROOT)}:{node.lineno}: {node.name}", (str(path), node.lineno)))
    return sites


class SourcesTest(unittest.TestCase):
    """The three lists parse to their known counts, each name once."""

    def test_the_catalog_holds_93_scenarios(self) -> None:
        ids = parse(CATALOG_ROW, CATALOG)
        self.assertEqual(len(ids), SCENARIO_COUNT)
        self.assertEqual(len(set(ids)), SCENARIO_COUNT)

    def test_the_specification_holds_32_rules(self) -> None:
        names = parse(SPEC_RULE, SPECIFICATION)
        self.assertEqual(len(names), RULE_COUNT)
        self.assertEqual(len(set(names)), RULE_COUNT)

    def test_the_specification_holds_35_invariants(self) -> None:
        names = parse(SPEC_INVARIANT, SPECIFICATION)
        self.assertEqual(len(names), INVARIANT_COUNT)
        self.assertEqual(len(set(names)), INVARIANT_COUNT)

    def test_every_tag_is_well_formed(self) -> None:
        malformed = read_tags()[1]
        self.assertEqual(malformed, [])

    def test_every_test_file_sits_where_load_tests_and_the_tags_look(self) -> None:
        # Both read double/tests/ alone, not its subdirectories, so a test file
        # in a subdirectory would be neither run nor counted.
        nested = [str(path.relative_to(TESTS)) for path in TESTS.rglob("*.py") if path.parent != TESTS]
        self.assertEqual(nested, [])

    def test_every_rule_names_its_trigger(self) -> None:
        triggers = rule_triggers()
        self.assertEqual(len(triggers), RULE_COUNT)
        self.assertEqual(sorted(name for name, trigger in triggers.items() if trigger is None), [])


class TagRunTest(unittest.TestCase):
    def test_every_tag_sits_on_a_test_that_ran_and_passed(self) -> None:
        require_full_suite(self)
        passed = sys.modules[__package__ or ""].PASSED
        unrun = [where for where, site in read_tags()[2] if site not in passed]
        self.assertEqual(
            unrun, [], "tags on a def the suite did not collect, or whose test did not pass; "
            "a tag sits directly above an undecorated test def"
        )


class TestDefRunTest(unittest.TestCase):
    def test_every_test_def_was_collected_and_outside_this_module_passed(self) -> None:
        require_full_suite(self)
        package = sys.modules[__package__ or ""]
        this_module = str(Path(__file__).resolve())
        missed = [
            where
            for where, site in defined_tests()
            if site not in package.COLLECTED or (site[0] != this_module and site not in package.PASSED)
        ]
        self.assertEqual(
            missed, [], "test defs the suite did not collect, or, outside test_coverage, whose test did not pass; "
            "a test def sits undecorated in a unittest.TestCase subclass in a test*.py module"
        )


class ScenarioCoverageTest(unittest.TestCase):
    def setUp(self) -> None:
        self.catalog = frozenset(parse(CATALOG_ROW, CATALOG))
        self.tagged = frozenset(read_tags()[0]["scenario"])

    def test_every_scenario_tag_names_a_catalog_row(self) -> None:
        self.assertEqual(sorted(self.tagged - self.catalog), [])

    def test_every_scenario_is_tagged(self) -> None:
        self.assertEqual(sorted(self.catalog - self.tagged), [])


class RuleCoverageTest(unittest.TestCase):
    def setUp(self) -> None:
        self.specified = frozenset(parse(SPEC_RULE, SPECIFICATION))
        self.registered = rules.SPECIFICATION.rule_names()
        self.triggers = rule_triggers()

    def test_every_table_is_a_specification_trigger(self) -> None:
        tables = {table.trigger for table in rules.SPECIFICATION.tables()}
        self.assertEqual(sorted(tables - set(self.triggers.values())), [])

    def test_every_registered_row_sits_in_its_triggers_table(self) -> None:
        misfiled = [
            f"{row.name} in the {table.trigger} table"
            for table in rules.SPECIFICATION.tables()
            for row in table.rows
            if self.triggers.get(row.name) != table.trigger
        ]
        self.assertEqual(sorted(misfiled), [])

    def test_every_registered_row_is_a_specification_rule(self) -> None:
        self.assertEqual(sorted(self.registered - self.specified), [])

    def test_every_rule_is_registered(self) -> None:
        self.assertEqual(sorted(self.specified - self.registered), [])

    def test_every_registered_row_fired_during_the_run(self) -> None:
        require_full_suite(self)
        never_fired = self.registered - rules.SPECIFICATION.fired()
        self.assertEqual(sorted(never_fired), [], "registered rows that never fired")


class InvariantCoverageTest(unittest.TestCase):
    """Every invariant has a registered check and a positive control."""

    def setUp(self) -> None:
        self.specified = frozenset(parse(SPEC_INVARIANT, SPECIFICATION))
        self.registered = frozenset(invariants.REGISTRY)
        self.controlled = frozenset(read_tags()[0]["invariant-control"])

    def test_every_registered_check_is_a_specification_invariant(self) -> None:
        self.assertEqual(sorted(self.registered - self.specified), [])

    def test_every_invariant_has_a_registered_check(self) -> None:
        self.assertEqual(sorted(self.specified - self.registered), [])

    def test_every_control_tag_names_a_specification_invariant(self) -> None:
        self.assertEqual(sorted(self.controlled - self.specified), [])

    def test_every_invariant_has_a_positive_control(self) -> None:
        self.assertEqual(sorted(self.specified - self.controlled), [])


class PassRecordTest(unittest.TestCase):
    """tests/__init__.py's wrapped run records a pass whether or not a result is given."""

    def test_a_run_given_no_result_records_its_pass_and_not_its_failure(self) -> None:
        from . import COLLECTED, PASSED, _record_passes

        class Passes(unittest.TestCase):
            def runTest(self) -> None:
                pass

        class Fails(unittest.TestCase):
            def runTest(self) -> None:
                self.fail("fails on purpose")

        recorded = {}
        for case_type in (Passes, Fails):
            code = case_type.runTest.__code__
            site = (str(Path(code.co_filename).resolve()), code.co_firstlineno)
            self.addCleanup(PASSED.discard, site)
            self.addCleanup(COLLECTED.discard, site)
            case = case_type()
            _record_passes(case)
            result = case.run()
            assert result is not None
            recorded[case_type.__name__] = (result.wasSuccessful(), site in PASSED)
        self.assertEqual(recorded, {"Passes": (True, True), "Fails": (False, False)})
