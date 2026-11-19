"""The double's tests, loaded in an order fixed by design.

`just python-double` runs `unittest discover -s double/tests -t double`, so
discovery imports this package as `tests` and calls `load_tests` below instead
of walking the directory itself. `load_tests` loads every `test*.py` module in
this directory, in name order, and then `test_coverage` last: its fired-rule
check reads the record the other modules' tests leave behind, so it must run
after all of them. `FULL_SUITE_LOADED` says that happened; `test_coverage`
reads it, and fails rather than skips when the recipe ran it without it.

`load_tests` also records each collected test's method site, file and first
line, in `COLLECTED`, and wraps the test's `run`, so `PASSED` ends up holding
the site of every test that ran and passed. `test_coverage` credits a tag only
when the def below it is in `PASSED`, so a tag on a test that was not
collected, was skipped or did not pass counts for nothing; and it fails on any
`def test` under this directory that is not in `COLLECTED`, or, outside
`test_coverage` itself, not in `PASSED`.
"""

import importlib
import inspect
import unittest
from pathlib import Path

LAST_MODULE = "test_coverage"

FULL_SUITE_LOADED = False

# Every test that ran and passed, as (file, first line) of its method: no
# failure, error, skip, expected failure or unexpected success was recorded
# while it ran. test_coverage credits a tag only when its def is in here.
PASSED: set[tuple[str, int]] = set()

# Every test load_tests collected, by the same site.
COLLECTED: set[tuple[str, int]] = set()

OUTCOMES = ("failures", "errors", "skipped", "expectedFailures", "unexpectedSuccesses")


def _cases(suite: unittest.TestSuite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from _cases(test)
        else:
            yield test


def _record_passes(test: unittest.TestCase) -> None:
    code = inspect.unwrap(getattr(type(test), test._testMethodName)).__code__
    site = (str(Path(code.co_filename).resolve()), code.co_firstlineno)
    COLLECTED.add(site)
    run = test.run

    def recording_run(result: unittest.TestResult | None = None) -> unittest.TestResult | None:
        if result is None:
            return run(result)
        before = tuple(len(getattr(result, name)) for name in OUTCOMES)
        outcome = run(result)
        if tuple(len(getattr(result, name)) for name in OUTCOMES) == before:
            PASSED.add(site)
        return outcome

    test.run = recording_run  # TestCase.__call__ calls self.run


def load_tests(
    loader: unittest.TestLoader, standard_tests: unittest.TestSuite, pattern: str | None
) -> unittest.TestSuite:
    global FULL_SUITE_LOADED
    here = Path(__file__).resolve().parent
    stems = sorted(path.stem for path in here.glob("test*.py"))
    if LAST_MODULE not in stems:
        raise RuntimeError(f"{LAST_MODULE}.py is missing from {here}")
    stems.remove(LAST_MODULE)
    stems.append(LAST_MODULE)
    suite = unittest.TestSuite()
    suite.addTests(standard_tests)
    for stem in stems:
        module = importlib.import_module(f"{__name__}.{stem}")
        suite.addTests(loader.loadTestsFromModule(module))
    for test in _cases(suite):
        _record_passes(test)
    FULL_SUITE_LOADED = True
    return suite
