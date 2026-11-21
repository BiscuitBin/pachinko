//// The entry module `gleam test` runs. `main` lists every test module, runs
//// the registry, then runs the coverage check over what the tests did. It
//// prints each failure and each coverage fault on its own line, then the
//// counts, and crashes, which exits non-zero, on any of them.

import config_test
import core_test
import coverage
import coverage_test
import gleam/int
import gleam/io
import gleam/list
import identity_test
import inspection_test
import invariants_test
import owner_test
import publish_test
import rules_test
import runner.{type Module, Module}
import runner_test

fn modules() -> List(Module) {
  [
    Module("runner_test", runner_test.tests()),
    Module("rules_test", rules_test.tests()),
    Module("core_test", core_test.tests()),
    Module("owner_test", owner_test.tests()),
    Module("config_test", config_test.tests()),
    Module("identity_test", identity_test.tests()),
    Module("publish_test", publish_test.tests()),
    Module("coverage_test", coverage_test.tests()),
    Module("inspection_test", inspection_test.tests()),
    Module("invariants_test", invariants_test.tests()),
  ]
}

pub fn main() -> Nil {
  let report = runner.run(modules(), "test")
  let faults = coverage.check(report)
  let failures =
    list.append(
      runner.failures(report),
      list.map(faults, fn(fault) { "COVERAGE " <> fault }),
    )
  list.each(failures, io.println_error)
  let #(ran, passed) = runner.counts(report)
  io.println(
    int.to_string(ran) <> " tests run, " <> int.to_string(passed) <> " passed",
  )
  io.println("coverage faults: " <> int.to_string(list.length(faults)))
  case failures {
    [] -> Nil
    _ -> panic as "the Gleam double's tests or its coverage check failed"
  }
}
