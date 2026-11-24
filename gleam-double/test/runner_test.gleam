//// The runner's own tests.

import gleam/erlang/process
import gleam/list
import gleam/string
import support/runner.{type Test, Failed, Module, Passed, Test}

pub fn tests() -> List(Test) {
  [
    Test("a passing test hands back what it fired", [], passing_test_passes),
    Test("a test that crashes fails", [], crashing_test_fails),
    Test("a test that does not return in time fails", [], slow_test_fails),
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

/// A test fails by crashing, so a crash must come back as `Failed`: were it
/// `Passed`, a failed assertion would credit the test's tags and rules. The
/// inner test kills its own process rather than panicking, which the runner
/// sees the same way, through its monitor, and which prints no crash report.
///
/// This test pins the reason. It cannot catch a runner that takes a crash
/// for a pass, since that runner would take this test's own failure for a
/// pass too; `runner.runner_faults`, which `runner.run` adds to its faults
/// outside the registry, catches that.
fn crashing_test_fails() -> List(String) {
  let inner =
    Test("inner", [], fn() {
      process.kill(process.self())
      ["SomeRule"]
    })
  let assert Failed(reason) = runner.run_one(inner, 1000)
  assert string.starts_with(reason, "crashed: ")
  []
}

/// A test that never returns is killed at the timeout and fails. It waits on
/// a subject no process sends to, so it can only time out.
fn slow_test_fails() -> List(String) {
  let inner =
    Test("inner", [], fn() {
      process.receive_forever(process.new_subject())
      ["SomeRule"]
    })
  assert runner.run_one(inner, 50) == Failed("timed out after 50 ms")
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
import support/runner.{type Module, Module}

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
