//// The coverage check's own tests, each on inputs made by hand.

import gleam/list
import gleam/string
import support/coverage.{type Inputs, Inputs}
import support/runner.{
  type Ran, type Tag, type Test, Failed, InvariantControl, Passed, Ran, Report,
  Scenario, Test,
}

pub fn tests() -> List(Test) {
  [
    Test(
      "a catalog row is `| S`, three digits, a space and a bar",
      [],
      scenario_rows,
    ),
    Test("a rule starts at column 0 and ends with a brace", [], rule_lines),
    Test("an invariant is found at any indent", [], invariant_lines),
    Test("a covered run has no fault", [], covered_run),
    Test("each count is compared with what was parsed", [], counts),
    Test("an entry a source holds twice is a fault", [], repeated_entries),
    Test("a tag for an id the catalog lacks is a fault", [], unknown_scenario),
    Test("a catalog id with no test is a fault", [], uncovered),
    Test("a failed test credits neither its tag nor a rule", [], failed_test),
    Test("a row the specification lacks is a fault", [], unknown_row),
    Test("a rule that is a row of no table is a fault", [], rule_without_row),
    Test("a fired name credits no rule that is not a row", [], fired_not_row),
    Test("a registered rule that never fired is a fault", [], unfired_row),
    Test("a name that is a row twice is a fault", [], repeated_row),
    Test("a fired name that no table holds is no fault", [], scratch_fired),
    Test("a name the specification lacks is a fault", [], unknown_invariant),
    Test("an invariant outside the registry is a fault", [], unregistered),
    Test("an invariant with no passed control is a fault", [], uncontrolled),
    Test("a source that cannot be read is a fault", [], unreadable_source),
    Test("a missing file is an error that names its path", [], missing_file),
  ]
}

/// The rows of a table whose first column is not an id, an id of two or
/// four digits, an id with a letter in it, an indented row and a row with
/// no space before its bar are no scenario.
fn scenario_rows() -> List(String) {
  let catalog =
    "# Scenarios

| Id | Description | Tags |
| --- | --- | --- |
| S001 | The first | core |
| S002 | The second, which names S777 | edge |
| S093 | The last | edge |

| Suite | Covers |
| S01 | Two digits |
| S0011 | Four digits |
| S00a | A letter |
  | S555 | Indented |
| S556| No space before the bar |
|S557 | No space after the bar |
S558 | No bar |
"
  assert coverage.scenario_ids(catalog) == ["S001", "S002", "S093"]
  []
}

/// A rule nested under something, a line that does not end with ` {`, such
/// as one with a space after its brace, a comment and a longer word are no
/// rule.
fn rule_lines() -> List(String) {
  assert coverage.rule_names(specification) == ["First", "Second"]
  assert coverage.rule_names("rule Spaced { \nrule Kept {\n") == ["Kept"]
  []
}

/// Most of the invariants sit indented inside an entity, so a prefix test
/// on the untrimmed line would miss them. A comment, a longer word and a
/// line that does not end with ` {` are no invariant.
fn invariant_lines() -> List(String) {
  assert coverage.invariant_names(specification) == ["Inner", "Deeper", "Outer"]
  []
}

/// The base every test below changes in one place: three scenarios, each
/// with a tagged test that passed; two rules, each a row that fired; three
/// invariants, each in the checker's registry and each with a control that
/// passed.
fn covered() -> Inputs {
  Inputs(
    catalog: Ok(catalog),
    specification: Ok(specification),
    rules: [#("Happened", ["First", "Second"])],
    invariants: ["Inner", "Deeper", "Outer"],
    report: Report(ran: list.append(scenarios(), controls()), faults: []),
    expected_scenarios: 3,
    expected_rules: 2,
    expected_invariants: 3,
  )
}

/// The base's tagged tests, one per scenario, each passed, which fire both
/// rows between them, and one untagged.
fn scenarios() -> List(Ran) {
  [
    passed("one", [Scenario("S001")], ["First"]),
    passed("two", [Scenario("S002")], ["Second"]),
    passed("three", [Scenario("S003")], []),
    passed("untagged", [], []),
  ]
}

/// The base's three controls, one per invariant, each passed.
fn controls() -> List(Ran) {
  [
    passed("inner", [InvariantControl("Inner")], []),
    passed("deeper", [InvariantControl("Deeper")], []),
    passed("outer", [InvariantControl("Outer")], []),
  ]
}

fn passed(name: String, tags: List(Tag), fired: List(String)) -> Ran {
  Ran("hand_test", name, tags, Passed(fired))
}

fn failed(name: String, tags: List(Tag)) -> Ran {
  Ran("hand_test", name, tags, Failed("crashed"))
}

fn with_ran(inputs: Inputs, ran: List(Ran)) -> Inputs {
  Inputs(..inputs, report: Report(ran:, faults: []))
}

fn and_ran(inputs: Inputs, more: List(Ran)) -> Inputs {
  with_ran(inputs, list.append(inputs.report.ran, more))
}

fn covered_run() -> List(String) {
  assert coverage.faults(covered()) == []
  []
}

/// A pattern that silently matches less, or a constant nobody moved, shows
/// as a count that differs.
fn counts() -> List(String) {
  assert coverage.faults(Inputs(..covered(), expected_scenarios: 4))
    == ["the catalog holds 3 scenarios, and the check expects 4"]
  assert coverage.faults(Inputs(..covered(), expected_rules: 1))
    == ["the specification holds 2 rules, and the check expects 1"]
  assert coverage.faults(Inputs(..covered(), expected_invariants: 20))
    == ["the specification holds 3 invariants, and the check expects 20"]
  []
}

/// A row written twice would keep a count right while another row is gone.
fn repeated_entries() -> List(String) {
  let twice =
    Inputs(
      ..covered(),
      catalog: Ok(catalog <> "| S001 | The first, again | core |\n"),
      specification: Ok(
        specification <> "rule First {\n}\n    invariant Outer {\n    }\n",
      ),
      expected_scenarios: 4,
      expected_rules: 3,
      expected_invariants: 4,
    )
  assert coverage.faults(twice)
    == [
      "the catalog holds S001 more than once",
      "the specification holds the rule First more than once",
      "the specification holds the invariant Outer more than once",
    ]
  []
}

/// A tag is checked against the catalog whether its test passed or not.
fn unknown_scenario() -> List(String) {
  let inputs =
    and_ran(covered(), [
      passed("stray", [Scenario("S999")], []),
      failed("broken", [Scenario("S998")]),
    ])
  assert coverage.faults(inputs)
    == [
      "hand_test.stray is tagged Scenario(\"S999\"), and the catalog holds no S999",
      "hand_test.broken is tagged Scenario(\"S998\"), and the catalog holds no S998",
    ]
  []
}

/// The base without its test for S003: the catalog still holds S003.
fn uncovered() -> List(String) {
  let inputs =
    with_ran(covered(), [
      passed("one", [Scenario("S001")], ["First"]),
      passed("two", [Scenario("S002")], ["Second"]),
      ..controls()
    ])
  assert coverage.faults(inputs)
    == ["S003 has no test tagged Scenario(\"S003\") that passed"]
  []
}

/// `one` is the only test tagged for S001 and the only one that fired
/// `First`. Once it fails, neither is covered.
fn failed_test() -> List(String) {
  let inputs =
    with_ran(covered(), [
      failed("one", [Scenario("S001")]),
      passed("two", [Scenario("S002")], ["Second"]),
      passed("three", [Scenario("S003")], []),
      ..controls()
    ])
  assert coverage.faults(inputs)
    == [
      "S001 has no test tagged Scenario(\"S001\") that passed",
      "First is a row of the Happened table, and no test that passed fired it",
    ]
  []
}

fn unknown_row() -> List(String) {
  let inputs =
    Inputs(..covered(), rules: [
      #("Happened", ["First", "Second", "NoSuchRule"]),
    ])
  assert coverage.faults(inputs)
    == [
      "the Happened table has a row named NoSuchRule, and the specification holds no rule NoSuchRule",
    ]
  []
}

fn rule_without_row() -> List(String) {
  assert coverage.faults(Inputs(..covered(), rules: [#("Happened", ["First"])]))
    == ["Second is a row of no table"]
  []
}

/// A rule is covered only as a row, so a test that fired its name does not
/// cover a rule no table holds.
fn fired_not_row() -> List(String) {
  let inputs =
    Inputs(..covered(), rules: [#("Happened", ["First"])])
    |> and_ran([passed("four", [], ["Second"])])
  assert coverage.faults(inputs) == ["Second is a row of no table"]
  []
}

fn unfired_row() -> List(String) {
  let inputs =
    with_ran(covered(), [
      passed("one", [Scenario("S001")], ["First"]),
      passed("two", [Scenario("S002")], []),
      passed("three", [Scenario("S003")], []),
      ..controls()
    ])
  assert coverage.faults(inputs)
    == [
      "Second is a row of the Happened table, and no test that passed fired it",
    ]
  []
}

/// A fired row is known by its name alone, so two rows with one name would
/// share one firing: the row that never fired is credited by the one that
/// did. In both runs below `First` fired once, and nothing else is wrong.
fn repeated_row() -> List(String) {
  let in_one_table =
    Inputs(..covered(), rules: [#("Happened", ["First", "First", "Second"])])
  assert coverage.faults(in_one_table)
    == ["First is a row more than once: Happened, Happened"]
  let in_two_tables =
    Inputs(..covered(), rules: [
      #("Happened", ["First", "Second"]),
      #("Other", ["First"]),
    ])
  assert coverage.faults(in_two_tables)
    == ["First is a row more than once: Happened, Other"]
  []
}

/// A test of the machinery fires the rows of a table of its own, which no
/// trigger reaches. The check asks which registered rows fired, and does
/// not ask where every fired name came from.
fn scratch_fired() -> List(String) {
  let inputs = and_ran(covered(), [passed("scratch", [], ["Scratch"])])
  assert coverage.faults(inputs) == []
  []
}

/// A control tag and a registry entry must each name an invariant the
/// specification holds, whether the tagged test passed or not.
fn unknown_invariant() -> List(String) {
  let inputs =
    Inputs(..covered(), invariants: ["Inner", "Deeper", "Outer", "Bogus"])
    |> and_ran([
      failed("control", [InvariantControl("NoSuchInvariant")]),
    ])
  assert coverage.faults(inputs)
    == [
      "hand_test.control is tagged InvariantControl(\"NoSuchInvariant\"), and the specification holds no invariant NoSuchInvariant",
      "the checker's registry holds Bogus, and the specification holds no invariant Bogus",
    ]
  []
}

fn unregistered() -> List(String) {
  let inputs = Inputs(..covered(), invariants: ["Inner", "Outer"])
  assert coverage.faults(inputs)
    == ["the invariant Deeper is not in the checker's registry"]
  []
}

/// A control that failed is no control, and a run with no control at all
/// lacks all three. An empty registry beside it is three faults more: the
/// invariant half cannot be met by the count alone.
fn uncontrolled() -> List(String) {
  let no_control = fn(name) {
    "the invariant "
    <> name
    <> " has no test tagged InvariantControl(\""
    <> name
    <> "\") that passed"
  }
  let tagged = scenarios()
  let inputs =
    with_ran(
      covered(),
      list.append(tagged, [
        passed("inner", [InvariantControl("Inner")], []),
        failed("deeper", [InvariantControl("Deeper")]),
      ]),
    )
  assert coverage.faults(inputs) == [no_control("Deeper"), no_control("Outer")]
  let bare = with_ran(covered(), tagged)
  assert coverage.faults(bare)
    == [no_control("Inner"), no_control("Deeper"), no_control("Outer")]
  let unregistered = fn(name) {
    "the invariant " <> name <> " is not in the checker's registry"
  }
  assert coverage.faults(Inputs(..bare, invariants: []))
    == [
      unregistered("Inner"),
      no_control("Inner"),
      unregistered("Deeper"),
      no_control("Deeper"),
      unregistered("Outer"),
      no_control("Outer"),
    ]
  []
}

/// A source that could not be read is one fault, its message, and the
/// halves that read the other source still run.
fn unreadable_source() -> List(String) {
  let no_catalog =
    Inputs(..covered(), catalog: Error("cannot read the catalog"))
  assert coverage.faults(
      Inputs(..no_catalog, rules: [#("Happened", ["First"])]),
    )
    == ["cannot read the catalog", "Second is a row of no table"]
  let no_specification =
    Inputs(..covered(), specification: Error("cannot read the specification"))
  let without_three =
    with_ran(no_specification, [
      passed("one", [Scenario("S001")], ["First"]),
      passed("two", [Scenario("S002")], ["Second"]),
      ..controls()
    ])
  assert coverage.faults(without_three)
    == [
      "S003 has no test tagged Scenario(\"S003\") that passed",
      "cannot read the specification",
    ]
  []
}

fn missing_file() -> List(String) {
  assert coverage.read("no/such/file.md")
    == Error("cannot read no/such/file.md: No such file or directory")
  let assert Ok(text) = coverage.read("gleam.toml")
  assert string.contains(text, "pachinko_double")
  []
}

const catalog =
  "| Id | Description | Tags |
| --- | --- | --- |
| S001 | The first | core |
| S002 | The second | edge |
| S003 | The third | edge |
"

const specification =
  "-- A specification made by hand.
-- rule Commented {
-- invariant Commented {

entity Pocket {
    key: String

    invariant Inner {
        this.key != null
    }
        invariant Deeper {
        }
    rule Nested {
    }
    invariants NotOne {
    }
    invariant NoBrace
}

rule First {
    when: Happened()
    ensures: true
}

rule Second {
    when: Happened()
}

rule NoBrace
rules NotOne {
ruleNotOne {

invariant Outer {
    true
}
"
