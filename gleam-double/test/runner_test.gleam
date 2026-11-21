//// The runner's own tests.

import gleam/list
import gleam/string
import runner.{type Test, Module, Passed, Test}

pub fn tests() -> List(Test) {
  [
    Test("a passing test hands back what it fired", [], passing_test_passes),
    Test(
      "a test module that main does not import is a fault",
      [],
      unimported_module_is_a_fault,
    ),
    Test("two modules that list the same test are no fault", [], shared_test),
  ]
}

fn passing_test_passes() -> List(String) {
  let inner = Test("inner", [], fn() { ["SomeRule"] })
  assert runner.run_one(inner, 1000) == Passed(["SomeRule"])
  []
}

/// `main` names each module beside its tests by hand, so a name can be listed
/// while the module's own tests never run. A module `main` imports and does
/// not use fails the build, so the import is what is checked. The source here
/// imports plainly, under an alias, with an unqualified name and from a
/// directory; a commented import, another module's longer name and the same
/// name in another directory are no import.
fn unimported_module_is_a_fault() -> List(String) {
  let source =
    "import gleam/io
import owner_test
import rules_test as rules
import nested/lease_test.{tests}
import publish_test_helpers
// import publish_test
import runner.{type Module, Module}

fn modules() -> List(Module) {
  [Module(\"pull_test\", owner_test.tests())]
}
"
  let on_disk = [
    "lease_test",
    "nested/lease_test",
    "owner_test",
    "publish_test",
    "pull_test",
    "rules_test",
  ]
  assert runner.import_faults(source, on_disk)
    == [
      "lease_test.gleam is on disk and main does not import it",
      "publish_test.gleam is on disk and main does not import it",
      "pull_test.gleam is on disk and main does not import it",
    ]
  []
}

/// Two modules may list one test, such as a control a support module hands
/// to both: each module's own tests still run. The faults that come from the
/// disk are left out, so the test does not depend on which files `test/`
/// holds.
fn shared_test() -> List(String) {
  let shared = Test("shared", [], fn() { [] })
  let listed = [
    Module("runner_test", [shared, Test("one", [], fn() { [] })]),
    Module("rules_test", [shared, Test("two", [], fn() { [] })]),
    Module("owner_test", [Test("three", [], fn() { [] })]),
  ]
  let faults =
    runner.registry_faults(listed, "test")
    |> list.filter(fn(fault) { !string.contains(fault, " is on disk and ") })
  assert faults == []
  []
}
