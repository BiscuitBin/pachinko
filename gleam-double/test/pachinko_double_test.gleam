//// The entry module `gleam test` runs. `main` lists every test module, runs
//// the registry, then runs the coverage check over what the tests did. It
//// prints each failure and each coverage fault on its own line, then the
//// counts, and crashes, which exits non-zero, on any of them.

import branch_a_test
import competing_test
import config_test
import consuming_test
import core_test
import coverage
import coverage_test
import decay_test
import expiry_test
import gleam/int
import gleam/io
import gleam/list
import guide_test
import identity_test
import inspection_test
import interface_test
import invariants_test
import lease_end_test
import lease_endings_test
import lease_test
import owner_test
import parking_test
import publish_test
import pull_test
import rules_test
import runner.{type Module, Module}
import runner_test
import settle_test

fn modules() -> List(Module) {
  [
    Module("runner_test", runner_test.tests()),
    Module("rules_test", rules_test.tests()),
    Module("core_test", core_test.tests()),
    Module("owner_test", owner_test.tests()),
    Module("competing_test", competing_test.tests()),
    Module("config_test", config_test.tests()),
    Module("identity_test", identity_test.tests()),
    Module("publish_test", publish_test.tests()),
    Module("pull_test", pull_test.tests()),
    Module("lease_test", lease_test.tests()),
    Module("settle_test", settle_test.tests()),
    Module("lease_end_test", lease_end_test.tests()),
    Module("lease_endings_test", lease_endings_test.tests()),
    Module("consuming_test", consuming_test.tests()),
    Module("expiry_test", expiry_test.tests()),
    Module("decay_test", decay_test.tests()),
    Module("branch_a_test", branch_a_test.tests()),
    Module("parking_test", parking_test.tests()),
    Module("guide_test", guide_test.tests()),
    Module("coverage_test", coverage_test.tests()),
    Module("interface_test", interface_test.tests()),
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
