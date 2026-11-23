"""The package's three interfaces: each namespace's list of names is pinned.

`pachinko_double` holds what a reader drives, `pachinko_double.harness` what a
test builds a queue with and checks it by, and `pachinko_double.exceptions`
every exception class the double defines. For each, `__all__` is exactly the
list below, in its order; each name is the same object as the one its defining
module holds; and the namespace binds no other public name and defines no
`__getattr__`, so a name cannot be reachable from a namespace that does not
list it. The three lists share no name.
"""

import importlib
import unittest
from types import ModuleType

# Each namespace's names in the order of its `__all__`, each with the module
# that defines it.
TOP_LEVEL = (
    ("Queue", "queue"),
    ("Config", "config"),
    ("Batch", "pull"),
    ("NoPocketAvailable", "pull"),
    ("TickReport", "queue"),
    ("LeaseView", "consuming"),
    ("HeldRecordView", "consuming"),
    ("Snapshot", "inspection"),
)

HARNESS = (
    ("Clock", "effects"),
    ("LeaseIds", "effects"),
    ("FakeClock", "effects"),
    ("SequentialLeaseIds", "effects"),
    ("check", "invariants"),
    ("Violation", "invariants"),
    ("ConfigValues", "inspection"),
)

# The refusals first, then what means the double itself is wrong.
EXCEPTIONS = (
    ("Refusal", "refusals"),
    ("ConfigRefused", "refusals"),
    ("PublisherAlreadyJoined", "refusals"),
    ("ConsumerAlreadyJoined", "refusals"),
    ("UnknownPublisher", "refusals"),
    ("UnknownConsumer", "refusals"),
    ("EqualRankRefused", "refusals"),
    ("PullRefused", "refusals"),
    ("LeaseFenced", "refusals"),
    ("RecordNotHeld", "refusals"),
    ("MarkConflict", "refusals"),
    ("ReleaseRefused", "refusals"),
    ("Defect", "rules"),
    ("GuardOverlap", "rules"),
    ("InvariantsBroken", "invariants"),
)

NAMESPACES = {
    "pachinko_double": TOP_LEVEL,
    "pachinko_double.harness": HARNESS,
    "pachinko_double.exceptions": EXCEPTIONS,
}


def public_names(namespace: ModuleType) -> list[str]:
    """Every name the namespace binds, but for the underscored ones and its own submodules."""
    return sorted(
        name
        for name, value in vars(namespace).items()
        if not name.startswith("_") and not is_own_submodule(namespace, name, value)
    )


def is_own_submodule(namespace: ModuleType, name: str, value: object) -> bool:
    """Whether the value is the submodule the import system binds under that name."""
    return isinstance(value, ModuleType) and value.__name__ == f"{namespace.__name__}.{name}"


class NamespaceTest(unittest.TestCase):
    def test_each_namespace_lists_exactly_its_names_in_order(self) -> None:
        for namespace, names in NAMESPACES.items():
            with self.subTest(namespace=namespace):
                listed = importlib.import_module(namespace).__all__
                self.assertEqual(listed, [name for name, _ in names])

    def test_the_lists_hold_8_7_and_15_names(self) -> None:
        self.assertEqual([len(names) for names in NAMESPACES.values()], [8, 7, 15])

    def test_each_name_is_its_defining_modules_object(self) -> None:
        for namespace, names in NAMESPACES.items():
            for name, module in names:
                with self.subTest(namespace=namespace, name=name):
                    defining = importlib.import_module(f"pachinko_double.{module}")
                    exported = getattr(importlib.import_module(namespace), name)
                    self.assertIs(exported, getattr(defining, name))
                    self.assertEqual(exported.__module__, defining.__name__)

    def test_each_namespace_binds_no_public_name_it_does_not_list(self) -> None:
        for namespace, names in NAMESPACES.items():
            with self.subTest(namespace=namespace):
                module = importlib.import_module(namespace)
                self.assertEqual(public_names(module), sorted(name for name, _ in names))
                # A module-level `__getattr__` would answer names `vars` does not hold.
                self.assertIsNone(vars(module).get("__getattr__"))

    def test_no_name_is_in_two_namespaces(self) -> None:
        every = [name for names in NAMESPACES.values() for name, _ in names]
        self.assertEqual(sorted(every), sorted(set(every)))
