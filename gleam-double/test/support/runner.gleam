//// The double's own test runner. Gleam's usual runner goes through EUnit,
//// which fixes no order and halts the VM, so nothing could run after the last
//// test. This one runs a typed registry in order, each test in its own
//// monitored process with a timeout, and hands back what every test did, so
//// that the coverage check can read it afterwards.

import gleam/erlang/process
import gleam/int
import gleam/list
import gleam/string
import simplifile

/// What a test pins. A tag is data on a registered test, so it cannot sit on
/// a test that never ran.
pub type Tag {
  Scenario(id: String)
  InvariantControl(name: String)
}

/// One test. `run` returns the names of the rules the test saw fire. It fails
/// by crashing, so only a test that passed can credit a rule.
pub type Test {
  Test(name: String, tags: List(Tag), run: fn() -> List(String))
}

/// One test module's registry, under the module's name.
pub type Module {
  Module(name: String, tests: List(Test))
}

pub type Outcome {
  Passed(fired: List(String))
  Failed(reason: String)
}

/// One test's result.
pub type Ran {
  Ran(module: String, name: String, tags: List(Tag), outcome: Outcome)
}

/// A whole run: every test's result, and the faults in the registry itself.
pub type Report {
  Report(ran: List(Ran), faults: List(String))
}

/// How long one test may take, in milliseconds.
pub const test_timeout = 5000

/// The entry module, whose `main` is the runner and holds no test.
const entry_module = "pachinko_double_test"

/// Runs every test of every module, in the order given. The faults are the
/// registry's and then the runner's own, from `runner_faults`.
pub fn run(modules: List(Module), test_directory: String) -> Report {
  let ran =
    list.flat_map(modules, fn(module) {
      list.map(module.tests, fn(t) {
        Ran(module.name, t.name, t.tags, run_one(t, test_timeout))
      })
    })
  Report(
    ran:,
    faults: list.append(
      registry_faults(modules, test_directory),
      runner_faults(),
    ),
  )
}

/// The runner's controls on itself: a test that crashes and a test that
/// never returns must each come back `Failed`. They run here, outside the
/// registry, because a registered test cannot show it: a runner that took a
/// crash for a pass would take that test's own failed assertion for a pass
/// too, and the run would stay green.
pub fn runner_faults() -> List(String) {
  let crashes =
    Test("crashes", [], fn() {
      process.kill(process.self())
      []
    })
  let hangs =
    Test("hangs", [], fn() {
      process.receive_forever(process.new_subject())
      []
    })
  list.flatten([
    case run_one(crashes, test_timeout) {
      Failed(_) -> []
      Passed(_) -> ["the runner took a test that crashed for a pass"]
    },
    case run_one(hangs, 50) {
      Failed(_) -> []
      Passed(_) -> ["the runner took a test that timed out for a pass"]
    },
  ])
}

/// Runs one test in its own process. The child waits at a gate until the
/// runner is monitoring it, so a test that crashes at once is still seen.
///
/// `Passed` means that `run` returned: the child sends what `run` hands back
/// and nothing else. A process the test started and did not wait for is not
/// watched once `run` has returned, so a test waits for what it starts.
pub fn run_one(t: Test, timeout: Int) -> Outcome {
  let ready = process.new_subject()
  let result = process.new_subject()
  let pid =
    process.spawn_unlinked(fn() {
      let gate = process.new_subject()
      process.send(ready, gate)
      process.receive_forever(gate)
      process.send(result, t.run())
    })
  let monitor = process.monitor(pid)
  let selector =
    process.new_selector()
    |> process.select_map(result, Passed)
    |> process.select_specific_monitor(monitor, fn(down) {
      Failed("crashed: " <> string.inspect(down))
    })
  case process.receive(ready, timeout) {
    Ok(gate) -> process.send(gate, Nil)
    Error(Nil) -> Nil
  }
  let outcome = case process.selector_receive(selector, timeout) {
    Ok(outcome) -> outcome
    Error(Nil) -> {
      process.kill(pid)
      Failed("timed out after " <> int.to_string(timeout) <> " ms")
    }
  }
  process.demonitor_process(monitor)
  outcome
}

/// The faults a registry can have before any test runs: no test at all, a
/// module with none, a name used twice, a `*_test.gleam` file on disk that
/// the registry does not list or that the entry module does not import, and
/// a listed module with no file.
///
/// `main` writes a module's name beside its tests by hand, so a listed name
/// does not show that the module's own `tests()` is the list that runs. The
/// import does. A module that `main` imports and does not use fails the
/// build, which takes the warning as an error, so the one module left to
/// catch here is the one `main` names and never imports. Two modules may
/// list the same test. A `main` that calls a module's `tests()` and drops
/// the list is not caught.
pub fn registry_faults(
  modules: List(Module),
  test_directory: String,
) -> List(String) {
  let listed = list.map(modules, fn(module) { module.name })
  let empty = case list.flat_map(modules, fn(module) { module.tests }) {
    [] -> ["the registry holds no test"]
    _ -> []
  }
  let empty_modules =
    list.filter(modules, fn(module) { module.tests == [] })
    |> list.map(fn(module) { module.name <> " lists no test" })
  let repeated =
    list.flat_map(modules, fn(module) {
      list.map(module.tests, fn(t) { module.name <> "." <> t.name })
    })
    |> repeats
    |> list.map(fn(name) { name <> " is registered more than once" })
  let repeated_modules =
    repeats(listed)
    |> list.map(fn(name) { name <> " is listed more than once" })
  let on_disk = case simplifile.get_files(test_directory) {
    Ok(files) -> Ok(test_modules(files, test_directory))
    Error(error) -> Error(simplifile.describe_error(error))
  }
  let disk = case on_disk {
    Error(reason) -> ["cannot list " <> test_directory <> ": " <> reason]
    Ok(found) ->
      list.flatten([
        list.filter(found, fn(name) { !list.contains(listed, name) })
          |> list.map(fn(name) {
            name <> ".gleam is on disk and not listed in main"
          }),
        list.filter(listed, fn(name) { !list.contains(found, name) })
          |> list.map(fn(name) { name <> " is listed and has no file" }),
        unimported(found, test_directory),
      ])
  }
  list.flatten([
    empty,
    empty_modules,
    repeated,
    repeated_modules,
    disk,
  ])
}

/// The test modules on disk that the entry module, read from disk, does not
/// import.
fn unimported(found: List(String), test_directory: String) -> List(String) {
  let entry = test_directory <> "/" <> entry_module <> ".gleam"
  case simplifile.read(entry) {
    Ok(source) -> import_faults(source, found)
    Error(error) -> [
      "cannot read " <> entry <> ": " <> simplifile.describe_error(error),
    ]
  }
}

/// The test modules on disk that the entry module's source does not import.
/// An import is a line that starts with `import`, so a commented one is none.
pub fn import_faults(source: String, on_disk: List(String)) -> List(String) {
  let imported = list.filter_map(string.split(source, "\n"), imported_module)
  list.filter(on_disk, fn(name) { !list.contains(imported, name) })
  |> list.map(fn(name) {
    name <> ".gleam is on disk and main does not import it"
  })
}

/// The module an import line names: what follows `import`, up to an alias or
/// a list of unqualified names.
fn imported_module(line: String) -> Result(String, Nil) {
  case line {
    "import " <> rest -> Ok(before(before(rest, " "), "."))
    _ -> Error(Nil)
  }
}

/// The text before a mark, or all of it where the mark is absent.
fn before(text: String, mark: String) -> String {
  case string.split_once(text, mark) {
    Ok(#(head, _)) -> head
    Error(Nil) -> text
  }
}

/// The module names of the `*_test.gleam` files under the test directory,
/// without the entry module, whose `main` is the runner and holds no test.
fn test_modules(files: List(String), test_directory: String) -> List(String) {
  let prefix = test_directory <> "/"
  list.filter_map(files, fn(file) {
    case
      string.starts_with(file, prefix),
      string.ends_with(file, "_test.gleam")
    {
      True, True ->
        Ok(
          file
          |> string.drop_start(string.length(prefix))
          |> string.drop_end(string.length(".gleam")),
        )
      _, _ -> Error(Nil)
    }
  })
  |> list.filter(fn(name) { name != entry_module })
  |> list.sort(string.compare)
}

/// The values that occur more than once, each named once.
fn repeats(names: List(String)) -> List(String) {
  list.filter(list.unique(names), fn(name) {
    list.count(names, fn(other) { other == name }) > 1
  })
}

/// The failures of a report, one line each: every failed test by name, then
/// every registry fault.
pub fn failures(report: Report) -> List(String) {
  let failed =
    list.filter_map(report.ran, fn(ran) {
      case ran.outcome {
        Passed(_) -> Error(Nil)
        Failed(reason) ->
          Ok("FAIL " <> ran.module <> "." <> ran.name <> ": " <> reason)
      }
    })
  list.append(failed, list.map(report.faults, fn(fault) { "FAULT " <> fault }))
}

/// How many tests ran, and how many of them passed.
pub fn counts(report: Report) -> #(Int, Int) {
  let passed =
    list.count(report.ran, fn(ran) {
      case ran.outcome {
        Passed(_) -> True
        Failed(_) -> False
      }
    })
  #(list.length(report.ran), passed)
}
