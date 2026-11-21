//// The coverage check, which `main` runs after the last test. It parses
//// three lists, the catalog's scenarios, the specification's rules and its
//// invariants, and names a fault for each entry that is neither covered nor
//// pending. `faults` is pure, so it is tested on inputs made by hand; `check`
//// reads the two files and the registries and calls it.

import gleam/int
import gleam/list
import gleam/string
import pachinko_double/core
import pachinko_double/invariants
import runner.{
  type Ran, type Report, type Tag, Failed, InvariantControl, Passed, Scenario,
}
import simplifile

/// The catalog and the specification, from `gleam-double/`, where the run
/// starts.
pub const catalog_path = "../docs/reference/testing.md"

pub const specification_path = "../docs/specs/pachinko.allium"

/// How many entries each pattern must find, so that a pattern which silently
/// matches less fails. A specification change that moves a count updates
/// the constant here and adds a spec note.
pub const expected_scenarios = 93

pub const expected_rules = 32

pub const expected_invariants = 35

/// The scenarios no test pins yet, in the catalog's order. Each ticket
/// takes off exactly what it lands.
pub const pending_scenarios = [
  "S002", "S006", "S007", "S021", "S024", "S025", "S026", "S034", "S037", "S043",
  "S044", "S045", "S046", "S048", "S054", "S056", "S058", "S059", "S060", "S061",
  "S062", "S063", "S064", "S065", "S066", "S067", "S069", "S071", "S072", "S073",
  "S074", "S075", "S076", "S077", "S078", "S079", "S080", "S081", "S082", "S083",
  "S087", "S088", "S089", "S091", "S092", "S093",
]

/// The rules no table holds yet, in the specification's order. A rule comes
/// off the list when it becomes a row.
pub const pending_rules = [
  "DeferBehindHeldVersion", "ReviveParkedPocket", "LeaseLapses",
  "LeaseExpiryParksPocket", "LeaseExpiryKeepsPocketAtFront",
  "LeaseExpiryIdlesEmptyPocket", "SupersedeUnsettledHeldVersion",
  "ConsumeAckedVersionQueueDeferred", "DeadLetterNackedVersionQueueDeferred",
  "SupersededRecordDecays", "DeadLetterDecays",
]

/// Everything the check reads. A file that could not be read is the `Error`
/// of its text, holding the fault's message.
pub type Inputs {
  Inputs(
    catalog: Result(String, String),
    specification: Result(String, String),
    /// Every table, as its trigger's name and its rows' names.
    rules: List(#(String, List(String))),
    /// The names in the checker's registry.
    invariants: List(String),
    report: Report,
    pending_scenarios: List(String),
    pending_rules: List(String),
    expected_scenarios: Int,
    expected_rules: Int,
    expected_invariants: Int,
  )
}

/// The faults of a run against the real catalog, specification, tables and
/// checker, and the pending lists above.
pub fn check(report: Report) -> List(String) {
  faults(Inputs(
    catalog: read(catalog_path),
    specification: read(specification_path),
    rules: core.registry(),
    invariants: list.map(invariants.registry(), fn(invariant) { invariant.name }),
    report:,
    pending_scenarios:,
    pending_rules:,
    expected_scenarios:,
    expected_rules:,
    expected_invariants:,
  ))
}

/// A file's text, or the fault that it could not be read.
pub fn read(path: String) -> Result(String, String) {
  case simplifile.read(path) {
    Ok(text) -> Ok(text)
    Error(error) ->
      Error("cannot read " <> path <> ": " <> simplifile.describe_error(error))
  }
}

/// The faults, one line each, each naming the id, the rule or the invariant
/// it concerns. None means the run is covered. A source that could not be
/// read is one fault, and the halves that read the other still run.
pub fn faults(inputs: Inputs) -> List(String) {
  let from_catalog = case inputs.catalog {
    Ok(catalog) -> scenario_faults(inputs, scenario_ids(catalog))
    Error(fault) -> [fault]
  }
  let from_specification = case inputs.specification {
    Ok(specification) ->
      list.append(
        rule_faults(inputs, rule_names(specification)),
        invariant_faults(inputs, invariant_names(specification)),
      )
    Error(fault) -> [fault]
  }
  list.append(from_catalog, from_specification)
}

/// The scenario half. A tag is checked against the catalog whatever its
/// test did, and credits its id only where the test passed.
fn scenario_faults(inputs: Inputs, ids: List(String)) -> List(String) {
  let tags = tagged(inputs.report.ran, scenario_tag)
  let covered = credited(tags)
  let unknown_tags =
    list.filter(tags, fn(tag) { !list.contains(ids, tag.1) })
    |> list.map(fn(tag) {
      test_name(tag.0)
      <> " is tagged Scenario(\""
      <> tag.1
      <> "\"), and the catalog holds no "
      <> tag.1
    })
  let uncovered =
    list.filter_map(ids, fn(id) {
      let tag = "Scenario(\"" <> id <> "\")"
      case
        list.contains(inputs.pending_scenarios, id),
        list.contains(covered, id)
      {
        True, True ->
          Ok(
            id
            <> " is in pending_scenarios, and a test tagged "
            <> tag
            <> " passed",
          )
        False, False ->
          Ok(
            id
            <> " is not in pending_scenarios, and no test tagged "
            <> tag
            <> " passed",
          )
        _, _ -> Error(Nil)
      }
    })
  let unknown_pending =
    list.filter(inputs.pending_scenarios, fn(id) { !list.contains(ids, id) })
    |> list.map(fn(id) {
      "pending_scenarios names " <> id <> ", and the catalog holds no " <> id
    })
  list.flatten([
    count_fault("the catalog", ids, "scenarios", inputs.expected_scenarios),
    list.map(repeats(ids), fn(id) {
      "the catalog holds " <> id <> " more than once"
    }),
    unknown_tags,
    uncovered,
    unknown_pending,
  ])
}

/// The rule half. The tables are the registry, so a rule counts as covered
/// once it is a row, and a row that is not pending must have fired in a
/// test that passed. A test reports a fired row by its name, so a name must
/// be one row: were it two, in one table or in two, the one that fired
/// would credit the other.
fn rule_faults(inputs: Inputs, names: List(String)) -> List(String) {
  let rows =
    list.flat_map(inputs.rules, fn(table) {
      list.map(table.1, fn(name) { #(name, table.0) })
    })
  let fired =
    list.flat_map(inputs.report.ran, fn(ran) {
      case ran.outcome {
        Passed(fired) -> fired
        Failed(_) -> []
      }
    })
  let unknown_rows =
    list.filter(rows, fn(row) { !list.contains(names, row.0) })
    |> list.map(fn(row) {
      "the "
      <> row.1
      <> " table has a row named "
      <> row.0
      <> ", and the specification holds no rule "
      <> row.0
    })
  let repeated_rows =
    list.map(repeats(list.map(rows, fn(row) { row.0 })), fn(name) {
      name
      <> " is a row more than once: "
      <> string.join(list.key_filter(rows, name), ", ")
    })
  let uncovered =
    list.filter_map(names, fn(name) {
      case
        list.contains(inputs.pending_rules, name),
        list.key_find(rows, name),
        list.contains(fired, name)
      {
        True, Ok(table), _ ->
          Ok(
            name
            <> " is in pending_rules and is a row of the "
            <> table
            <> " table",
          )
        False, Error(Nil), _ ->
          Ok(name <> " is not in pending_rules and is a row of no table")
        False, Ok(table), False ->
          Ok(
            name
            <> " is a row of the "
            <> table
            <> " table and not in pending_rules, and no test that passed fired it",
          )
        _, _, _ -> Error(Nil)
      }
    })
  let unknown_pending =
    list.filter(inputs.pending_rules, fn(name) { !list.contains(names, name) })
    |> list.map(fn(name) {
      "pending_rules names "
      <> name
      <> ", and the specification holds no rule "
      <> name
    })
  list.flatten([
    count_fault("the specification", names, "rules", inputs.expected_rules),
    list.map(repeats(names), fn(name) {
      "the specification holds the rule " <> name <> " more than once"
    }),
    unknown_rows,
    repeated_rows,
    uncovered,
    unknown_pending,
  ])
}

/// The invariant half. The count must be the one expected, a control tag
/// and a registry entry must each name an invariant the specification holds,
/// and each invariant must be in the checker's registry and have a positive
/// control, tagged `InvariantControl`, that passed. It has no pending list
/// and no switch: no invariant is ever pending.
fn invariant_faults(inputs: Inputs, names: List(String)) -> List(String) {
  let tags = tagged(inputs.report.ran, control_tag)
  let controlled = credited(tags)
  let unknown_tags =
    list.filter(tags, fn(tag) { !list.contains(names, tag.1) })
    |> list.map(fn(tag) {
      test_name(tag.0)
      <> " is tagged InvariantControl(\""
      <> tag.1
      <> "\"), and the specification holds no invariant "
      <> tag.1
    })
  let unknown_entries =
    list.filter(inputs.invariants, fn(name) { !list.contains(names, name) })
    |> list.map(fn(name) {
      "the checker's registry holds "
      <> name
      <> ", and the specification holds no invariant "
      <> name
    })
  let uncovered =
    list.flat_map(names, fn(name) {
      let unregistered = case list.contains(inputs.invariants, name) {
        True -> []
        False -> [
          "the invariant " <> name <> " is not in the checker's registry",
        ]
      }
      let uncontrolled = case list.contains(controlled, name) {
        True -> []
        False -> [
          "the invariant "
          <> name
          <> " has no test tagged InvariantControl(\""
          <> name
          <> "\") that passed",
        ]
      }
      list.append(unregistered, uncontrolled)
    })
  list.flatten([
    count_fault(
      "the specification",
      names,
      "invariants",
      inputs.expected_invariants,
    ),
    list.map(repeats(names), fn(name) {
      "the specification holds the invariant " <> name <> " more than once"
    }),
    unknown_tags,
    unknown_entries,
    uncovered,
  ])
}

/// The fault of a list whose length is not the one expected.
fn count_fault(
  source: String,
  found: List(String),
  kind: String,
  expected: Int,
) -> List(String) {
  case list.length(found) == expected {
    True -> []
    False -> [
      source
      <> " holds "
      <> int.to_string(list.length(found))
      <> " "
      <> kind
      <> ", and the check expects "
      <> int.to_string(expected),
    ]
  }
}

/// Every tag of one kind in a run, each beside the test that carries it.
fn tagged(
  ran: List(Ran),
  pick: fn(Tag) -> Result(String, Nil),
) -> List(#(Ran, String)) {
  list.flat_map(ran, fn(ran) {
    list.filter_map(ran.tags, pick) |> list.map(fn(name) { #(ran, name) })
  })
}

fn scenario_tag(tag: Tag) -> Result(String, Nil) {
  case tag {
    Scenario(id) -> Ok(id)
    InvariantControl(_) -> Error(Nil)
  }
}

fn control_tag(tag: Tag) -> Result(String, Nil) {
  case tag {
    InvariantControl(name) -> Ok(name)
    Scenario(_) -> Error(Nil)
  }
}

/// What the tags of the tests that passed name. A test that failed credits
/// nothing.
fn credited(tags: List(#(Ran, String))) -> List(String) {
  list.filter_map(tags, fn(tag) {
    case { tag.0 }.outcome {
      Passed(_) -> Ok(tag.1)
      Failed(_) -> Error(Nil)
    }
  })
}

fn test_name(ran: Ran) -> String {
  ran.module <> "." <> ran.name
}

/// The values that occur more than once, each named once.
fn repeats(names: List(String)) -> List(String) {
  list.filter(list.unique(names), fn(name) {
    list.count(names, fn(other) { other == name }) > 1
  })
}

/// The ids of the catalog's rows: the lines that start with `| S` and three
/// digits, followed by a space and a `|`.
pub fn scenario_ids(catalog: String) -> List(String) {
  list.filter_map(string.split(catalog, "\n"), fn(line) {
    case line {
      "| S" <> rest ->
        case string.split_once(rest, " |") {
          Ok(#(number, _)) ->
            case three_digits(number) {
              True -> Ok("S" <> number)
              False -> Error(Nil)
            }
          Error(Nil) -> Error(Nil)
        }
      _ -> Error(Nil)
    }
  })
}

fn three_digits(text: String) -> Bool {
  let graphemes = string.to_graphemes(text)
  list.length(graphemes) == 3
  && list.all(graphemes, fn(grapheme) {
    list.contains(["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"], grapheme)
  })
}

/// The names of the specification's rules: the lines that start at column 0
/// with the word `rule` and a space, and end with a space and `{`.
pub fn rule_names(specification: String) -> List(String) {
  list.filter_map(string.split(specification, "\n"), declared(_, "rule "))
}

/// The names of the specification's invariants: the lines whose first word,
/// after any leading spaces, is `invariant` and which end with a space and
/// `{`. Most sit indented inside an entity.
pub fn invariant_names(specification: String) -> List(String) {
  list.filter_map(string.split(specification, "\n"), fn(line) {
    declared(without_leading_spaces(line), "invariant ")
  })
}

/// The name a declaration line gives: what stands between the keyword and
/// the ` {` that ends the line.
fn declared(line: String, keyword: String) -> Result(String, Nil) {
  case string.starts_with(line, keyword) && string.ends_with(line, " {") {
    True ->
      Ok(
        line
        |> string.drop_start(string.length(keyword))
        |> string.drop_end(string.length(" {")),
      )
    False -> Error(Nil)
  }
}

fn without_leading_spaces(line: String) -> String {
  case line {
    " " <> rest -> without_leading_spaces(rest)
    _ -> line
  }
}
