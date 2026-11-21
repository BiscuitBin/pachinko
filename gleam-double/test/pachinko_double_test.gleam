//// The entry module `gleam test` runs. `main` lists every test module, runs
//// the registry, prints each failure by name and the counts, and crashes,
//// which exits non-zero, on any failure.

import config_test
import gleam/int
import gleam/io
import gleam/list
import identity_test
import owner_test
import rules_test
import runner.{type Module, Module}
import runner_test

fn modules() -> List(Module) {
  [
    Module("runner_test", runner_test.tests()),
    Module("rules_test", rules_test.tests()),
    Module("owner_test", owner_test.tests()),
    Module("config_test", config_test.tests()),
    Module("identity_test", identity_test.tests()),
  ]
}

pub fn main() -> Nil {
  let report = runner.run(modules(), "test")
  let failures = runner.failures(report)
  list.each(failures, io.println_error)
  let #(ran, passed) = runner.counts(report)
  io.println(
    int.to_string(ran) <> " tests run, " <> int.to_string(passed) <> " passed",
  )
  case failures {
    [] -> Nil
    _ -> panic as "the Gleam double's tests failed"
  }
}
