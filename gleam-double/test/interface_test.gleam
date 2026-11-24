//// The pin on the package's interface. `items` reads a module's source and
//// returns its public items, each with its constructors and its `@internal`
//// mark; `imports_effects` says whether a module imports `gleam/erlang` or
//// `gleam/otp`, and `reaches_effects` whether it does or holds an
//// `@external` line; `imported_modules` names the module each import line
//// takes. All four are pure, so the controls run them on text made by hand.
//// The assertions read the tree from `gleam-double/`, where the run starts:
//// the root's items are exactly the seventeen of the interface, the root is
//// the only module under `src/` that imports an effect or binds an external
//// function, and `src/` holds at its top level only the root and its
//// directory. The two vocabulary modules of the replies, `failure` and
//// `answers`, hold exactly the reply types; `failure` imports nothing, and
//// `answers` imports only `model` and `time`. The witness, the walkthrough
//// in `test/guide_test.gleam`, imports no engine module and no rule module.
//// `test/` holds at its top level only the test modules and the directory
//// `support`, which holds exactly the coverage check, the fakes and the
//// runner.
////
//// The layout splits the interface from the engine: `src/pachinko_double/`
//// holds the eight vocabulary modules and the directory `internal`, which
//// holds the engine and the specification's rules. Each vocabulary module
//// holds exactly its interface unmarked and the rest marked `@internal`, and
//// only `model`, `inspection` and `invariants` carry a mark; none imports a
//// module under `internal/`, and no module under `src/` imports a module
//// under an alias. The names that are `pub` only for the tests are
//// the six of `test_only`, which the guide's table lists too: each is a
//// `pub fn` under `internal/` or marked that no other module under `src/`
//// names, and every other `pub fn` and `pub const` under `internal/` or
//// marked is named by some other module under `src/`. The scan covers
//// functions and constants, not types: a marked type that only the tests
//// name, such as `invariants.Invariant`, is not seen.
////
//// The parser reads text, not the compiler's result:
//// - an item is a line that starts at column 0 with `pub fn`, `pub const`,
////   `pub type` or `pub opaque type`, and its name is the word that follows,
////   which ends at the first character that is not a letter, a digit or `_`;
//// - a `pub type` line that ends in `{` opens a body, which runs to the next
////   line that is `}`; a `pub type` line that does not is an alias;
//// - a type's constructors are the lines of its body that start with exactly
////   two spaces and an uppercase letter; an opaque type and an alias have
////   none;
//// - an item is internal when the line directly above its `pub` line is
////   `@internal`, and a constructor is internal when its type is;
//// - an `@external` line starts at column 0 with `@external`;
//// - an import line starts with `import `, and the module it names is the
////   text after that, up to the first space or `.`; its name list is read
////   without its comments, each line up to its first `//`;
//// - a qualified use `module.function` is read on each line up to its first
////   `//`, so a comment, whole or trailing, names nothing;
//// - a mark is a line that is `@internal` once trimmed, wherever it sits;
//// - a directory's listing leaves out the entries whose names start with
////   `.`, such as `.DS_Store` or an editor's swap file;
//// - a row of the guide's test-only table is a line under its heading that
////   starts with `|` once its leading spaces are dropped; past the header
////   and the separator, a row's name is its first cell, trimmed and without
////   its backticks.
////
//// The text a control reads is a list of lines or a string written with
//// `\n` escapes, so that no line of this file other than its own imports
//// starts with `import`.

import gleam/list
import gleam/result
import gleam/string
import simplifile
import support/coverage
import support/runner.{type Test, Test}

/// One public item of a module: its name, its constructors in the body's
/// order, and whether it is marked `@internal`.
pub type Item {
  Item(name: String, constructors: List(String), internal: Bool)
}

/// The root module and the source directory, from `gleam-double/`.
pub const root_path = "src/pachinko_double.gleam"

pub const source_directory = "src"

/// The two vocabulary modules of the replies, and the witness, from
/// `gleam-double/`.
pub const failure_path = "src/pachinko_double/failure.gleam"

pub const answers_path = "src/pachinko_double/answers.gleam"

pub const witness_path = "test/guide_test.gleam"

/// The vocabulary modules' directory and the engine's, from `gleam-double/`.
pub const vocabulary_directory = "src/pachinko_double"

pub const internal_directory = "src/pachinko_double/internal"

pub fn tests() -> List(Test) {
  [
    Test(
      "an item is a pub line at column 0, with its constructors and its mark",
      [],
      reads_items,
    ),
    Test("adding a pub item changes the items", [], item_added),
    Test("removing a pub item changes the items", [], item_removed),
    Test("dropping an @internal line changes the items", [], mark_dropped),
    Test("adding a constructor changes the items", [], constructor_added),
    Test(
      "an effect import starts with import gleam/erlang or gleam/otp",
      [],
      reads_effect_imports,
    ),
    Test(
      "a line that starts with @external reaches an effect",
      [],
      reads_externals,
    ),
    Test("the root holds exactly the interface's items", [], root_items),
    Test(
      "the root is the only module under src/ that imports an effect or holds an @external",
      [],
      effect_importers,
    ),
    Test("src/ holds only the root and its directory", [], source_top_level),
    Test(
      "an import line names the module up to the first space or dot",
      [],
      reads_imports,
    ),
    Test(
      "the witness allows gleam/, the root, the vocabulary and test support",
      [],
      reads_witness,
    ),
    Test("failure holds exactly Failure and Cause", [], failure_items),
    Test("failure imports nothing", [], failure_imports),
    Test("answers holds exactly the five reply types", [], answers_items),
    Test("answers imports only model and time", [], answers_imports),
    Test(
      "the walkthrough imports no engine module and no rule module",
      [],
      witness_imports,
    ),
    Test("a mark gained or lost moves an item between the halves", [], marks),
    Test(
      "an internal import starts with import pachinko_double/internal/",
      [],
      reads_internal_imports,
    ),
    Test(
      "an aliased import is an import line with as, or a closing } as",
      [],
      reads_aliased_imports,
    ),
    Test(
      "src/pachinko_double/ holds the eight vocabulary modules and internal/",
      [],
      vocabulary_top_level,
    ),
    Test("internal/ holds exactly the eleven modules", [], internal_top_level),
    Test(
      "a mark is a line that is @internal once its leading spaces are dropped",
      [],
      reads_mark_lines,
    ),
    Test(
      "only model, inspection and invariants carry marks: 12, 1 and 2",
      [],
      marks_per_module,
    ),
    Test("config holds its interface, and nothing marked", [], fn() {
      holds(config_vocabulary())
    }),
    Test("time holds its interface, and nothing marked", [], fn() {
      holds(time_vocabulary())
    }),
    Test("model holds its interface, and its twelve marked items", [], fn() {
      holds(model_vocabulary())
    }),
    Test("refusals holds its interface, and nothing marked", [], fn() {
      holds(refusals_vocabulary())
    }),
    Test("inspection holds its interface, and its one marked item", [], fn() {
      holds(inspection_vocabulary())
    }),
    Test("invariants holds its interface, and its two marked items", [], fn() {
      holds(invariants_vocabulary())
    }),
    Test(
      "no vocabulary module imports a module under internal/",
      [],
      vocabulary_imports,
    ),
    Test(
      "no module under src/ imports a module under an alias",
      [],
      no_aliased_imports,
    ),
    Test(
      "a table's names are its rows' first cells, under its heading",
      [],
      reads_table,
    ),
    Test(
      "a value is a pub fn or pub const line, with its mark",
      [],
      reads_values,
    ),
    Test(
      "a module names m.f qualified outside a comment, or unqualified in its import",
      [],
      reads_names,
    ),
    Test(
      "the test-only scan fails on a new user, a new seam and a list that differs",
      [],
      scan,
    ),
    Test(
      "the guide's table lists exactly the names pub only for the tests",
      [],
      guide_table,
    ),
    Test(
      "the names pub only for the tests are the six, and no other name is",
      [],
      test_only_names,
    ),
    Test(
      "a top level that holds a module that is not a test, a directory named like one, or no support, differs",
      [],
      reads_test_top_level,
    ),
    Test(
      "a support listing with a fourth module, or one missing, differs",
      [],
      reads_support_listing,
    ),
    Test(
      "a listing leaves out the entries whose names start with a dot",
      [],
      reads_hidden_entries,
    ),
    Test(
      "test/ holds at its top level only test modules and support/",
      [],
      test_top_level,
    ),
    Test(
      "test/support/ holds exactly coverage, harness and runner",
      [],
      support_top_level,
    ),
  ]
}

/// A module's public items, in the file's order.
pub fn items(source: String) -> List(Item) {
  collect(string.split(source, "\n"), "", [])
}

fn collect(
  lines: List(String),
  previous: String,
  found: List(Item),
) -> List(Item) {
  case lines {
    [] -> list.reverse(found)
    [line, ..rest] -> {
      let found = case declared(line) {
        Ok(#(name, opens_body)) -> {
          let constructors = case opens_body {
            True -> body_constructors(rest, [])
            False -> []
          }
          [
            Item(name:, constructors:, internal: previous == "@internal"),
            ..found
          ]
        }
        Error(Nil) -> found
      }
      collect(rest, line, found)
    }
  }
}

/// The name an item line declares, and whether it opens a body whose
/// constructors count: only a `pub type` line that ends in `{` does.
fn declared(line: String) -> Result(#(String, Bool), Nil) {
  case line {
    "pub fn " <> rest | "pub const " <> rest | "pub opaque type " <> rest ->
      Ok(#(word(rest), False))
    "pub type " <> rest -> Ok(#(word(rest), string.ends_with(line, "{")))
    _ -> Error(Nil)
  }
}

/// The constructors of a body, from its first line to the next line that is
/// `}`.
fn body_constructors(lines: List(String), found: List(String)) -> List(String) {
  case lines {
    [] | ["}", ..] -> list.reverse(found)
    [line, ..rest] ->
      case line {
        "  " <> constructor ->
          case starts_uppercase(constructor) {
            True -> body_constructors(rest, [word(constructor), ..found])
            False -> body_constructors(rest, found)
          }
        _ -> body_constructors(rest, found)
      }
  }
}

/// The leading run of letters, digits and `_`.
fn word(text: String) -> String {
  text
  |> string.to_graphemes
  |> list.take_while(fn(grapheme) {
    string.contains(
      "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_",
      grapheme,
    )
  })
  |> string.concat
}

fn starts_uppercase(text: String) -> Bool {
  case string.first(text) {
    Ok(grapheme) -> string.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ", grapheme)
    Error(Nil) -> False
  }
}

/// Whether a module's source holds a line that starts with
/// `import gleam/erlang` or `import gleam/otp`.
pub fn imports_effects(source: String) -> Bool {
  list.any(string.split(source, "\n"), fn(line) {
    string.starts_with(line, "import gleam/erlang")
    || string.starts_with(line, "import gleam/otp")
  })
}

/// Whether a module's source reaches an effect: it imports `gleam/erlang` or
/// `gleam/otp`, or holds a line that starts with `@external`, which binds a
/// function to Erlang's own.
pub fn reaches_effects(source: String) -> Bool {
  imports_effects(source)
  || list.any(string.split(source, "\n"), string.starts_with(_, "@external"))
}

/// The module each line of a source that starts with `import ` names, in the
/// file's order: the text after `import `, up to the first space or `.`.
pub fn imported_modules(source: String) -> List(String) {
  list.filter_map(string.split(source, "\n"), fn(line) {
    case line {
      "import " <> rest -> Ok(module_name(rest))
      _ -> Error(Nil)
    }
  })
}

fn module_name(text: String) -> String {
  text
  |> string.to_graphemes
  |> list.take_while(fn(grapheme) { grapheme != " " && grapheme != "." })
  |> string.concat
}

/// Whether the witness may import a module: one under `gleam/`,
/// `simplifile`, the support modules `support/harness` and `support/runner`,
/// the root, or one of the eight vocabulary modules.
pub fn witness_allows(module: String) -> Bool {
  string.starts_with(module, "gleam/")
  || list.contains(
    [
      "simplifile", "support/harness", "support/runner", "pachinko_double",
      "pachinko_double/config", "pachinko_double/time", "pachinko_double/model",
      "pachinko_double/refusals", "pachinko_double/failure",
      "pachinko_double/answers", "pachinko_double/inspection",
      "pachinko_double/invariants",
    ],
    module,
  )
}

/// The modules a source imports that the witness may not.
pub fn witness_faults(source: String) -> List(String) {
  list.filter(imported_modules(source), fn(module) { !witness_allows(module) })
}

/// A module made by hand: a function, a constant, a type with two
/// constructors, an alias, an opaque type and an internal type. The lines
/// around them are no item and no constructor: a private function, an
/// indented `pub`, a comment, a field line and a line indented further.
fn module() -> List(String) {
  [
    "//// A module made by hand.", "", "pub const timeout = 10", "",
    "/// pub fn commented() -> Nil", "fn private() -> Nil {", "  Nil", "}", "",
    "pub fn start(limit: Int) -> Int {", "  pub fn indented() -> Nil", "  limit",
    "}", "", "pub type Shape {", "  Circle(radius: Int)", "  Square(",
    "    Side(Int),", "  )", "}", "", "pub type Alias =", "  Result(Shape, Nil)",
    "", "pub opaque type Hidden {", "  Hidden(value: Int)", "}", "", "@internal",
    "pub type Seam {", "  Seam(", "    lowercase: Int,", "  )", "}",
  ]
}

/// What `items` reads from `module()`.
fn module_items() -> List(Item) {
  [
    Item("timeout", [], False),
    Item("start", [], False),
    Item("Shape", ["Circle", "Square"], False),
    Item("Alias", [], False),
    Item("Hidden", [], False),
    Item("Seam", ["Seam"], True),
  ]
}

fn reads_items() -> List(String) {
  assert items(string.join(module(), "\n")) == module_items()
  []
}

fn item_added() -> List(String) {
  let source =
    list.append(module(), ["", "pub fn added() -> Nil {", "  Nil", "}"])
  assert items(string.join(source, "\n"))
    == list.append(module_items(), [Item("added", [], False)])
  []
}

fn item_removed() -> List(String) {
  let source =
    list.map(module(), fn(line) {
      case line {
        "pub const timeout = 10" -> "const timeout = 10"
        _ -> line
      }
    })
  assert items(string.join(source, "\n")) == list.drop(module_items(), 1)
  []
}

fn mark_dropped() -> List(String) {
  let source = list.filter(module(), fn(line) { line != "@internal" })
  assert items(string.join(source, "\n"))
    == list.map(module_items(), fn(item) { Item(..item, internal: False) })
  []
}

fn constructor_added() -> List(String) {
  let source =
    list.flat_map(module(), fn(line) {
      case line {
        "  Circle(radius: Int)" -> [line, "  Triangle"]
        _ -> [line]
      }
    })
  assert items(string.join(source, "\n"))
    == list.map(module_items(), fn(item) {
      case item.name {
        "Shape" -> Item(..item, constructors: ["Circle", "Triangle", "Square"])
        _ -> item
      }
    })
  []
}

/// An indented import, a comment and a longer module name that only starts
/// alike are each read as written: only a line that starts with the prefix
/// counts.
fn reads_effect_imports() -> List(String) {
  assert imports_effects("import gleam/erlang/process.{type Subject}\n")
  assert imports_effects("import gleam/list\nimport gleam/otp/actor\n")
  assert !imports_effects("import gleam/list\nimport gleam/option\n")
  assert !imports_effects("  import gleam/otp/actor\n// import gleam/erlang\n")
  []
}

/// A line that starts with `@external` reaches an effect with no effect
/// import; an indented one and a comment that holds it are read as written,
/// and an effect import still counts.
fn reads_externals() -> List(String) {
  let external =
    "@external(erlang, \"os\", \"system_time\")\npub fn now() -> Int\n"
  assert !imports_effects(external)
  assert reaches_effects(external)
  assert reaches_effects("import gleam/erlang/process\n")
  assert !reaches_effects(
    "  @external(erlang, \"os\", \"system_time\")\n// @external(erlang, \"os\", \"getenv\")\nimport gleam/list\n",
  )
  []
}

/// The interface's seventeen items, by name: none is internal, and only
/// `Outcome` and `PartialTick` have a constructor, of the type's own name.
fn interface() -> List(Item) {
  [
    Item("start", [], False),
    Item("Message", [], False),
    Item("call_timeout", [], False),
    Item("publisher_joins", [], False),
    Item("publish", [], False),
    Item("consumer_joins", [], False),
    Item("pull", [], False),
    Item("renew", [], False),
    Item("ack", [], False),
    Item("nack", [], False),
    Item("release", [], False),
    Item("lease_view", [], False),
    Item("tick", [], False),
    Item("Reply", [], False),
    Item("Outcome", ["Outcome"], False),
    Item("PartialTick", ["PartialTick"], False),
    Item("inspect", [], False),
  ]
}

fn by_name(items: List(Item)) -> List(Item) {
  list.sort(items, fn(a, b) { string.compare(a.name, b.name) })
}

/// The comparison ignores order: the file's order is not the header's
/// grouping, and moving a function is not a change of interface.
fn root_items() -> List(String) {
  let found =
    coverage.read(root_path)
    |> result.map(fn(source) { by_name(items(source)) })
  assert found == Ok(by_name(interface()))
  []
}

fn effect_importers() -> List(String) {
  let importers = case simplifile.get_files(source_directory) {
    Ok(files) ->
      list.filter(files, fn(file) { string.ends_with(file, ".gleam") })
      |> list.filter_map(fn(file) {
        case coverage.read(file) {
          Ok(source) ->
            case reaches_effects(source) {
              True -> Ok(file)
              False -> Error(Nil)
            }
          Error(fault) -> Ok(fault)
        }
      })
      |> list.sort(string.compare)
    Error(error) -> [
      "cannot list "
      <> source_directory
      <> ": "
      <> simplifile.describe_error(error),
    ]
  }
  assert importers == [root_path]
  []
}

fn source_top_level() -> List(String) {
  let entries =
    simplifile.read_directory(source_directory)
    |> result.map(listing)
  assert entries == Ok(["pachinko_double", "pachinko_double.gleam"])
  assert simplifile.is_directory(source_directory <> "/pachinko_double")
    == Ok(True)
  []
}

/// Import lines made by hand. The engine's `core` is taken whole, with
/// unqualified names and under an alias, and a module is taken across
/// several lines. An indented import and a comment are no import line.
fn import_lines() -> List(String) {
  [
    "//// A test module made by hand.", "", "import gleam/list",
    "import gleam/option.{type Option, None}",
    "import support/harness.{type Double}", "import pachinko_double",
    "import pachinko_double.{type Reply, Outcome}",
    "import pachinko_double/core", "import pachinko_double/core.{TickReport}",
    "import pachinko_double/core as engine", "import pachinko_double/model.{",
    "  type Mark,", "}", "  import pachinko_double/domain",
    "// import pachinko_double/rules", "import support/runner.{type Test, Test}",
    "import simplifile",
  ]
}

fn reads_imports() -> List(String) {
  assert imported_modules(string.join(import_lines(), "\n"))
    == [
      "gleam/list", "gleam/option", "support/harness", "pachinko_double",
      "pachinko_double", "pachinko_double/core", "pachinko_double/core",
      "pachinko_double/core", "pachinko_double/model", "support/runner",
      "simplifile",
    ]
  []
}

/// The three lines that take `core` are each a fault; a module whose name
/// only starts like an allowed one is not allowed.
fn reads_witness() -> List(String) {
  assert witness_faults(string.join(import_lines(), "\n"))
    == ["pachinko_double/core", "pachinko_double/core", "pachinko_double/core"]
  assert !witness_allows("pachinko_double/configuration")
  assert !witness_allows("pachinko_double/domain")
  assert !witness_allows("pachinko_double/internal/core")
  assert !witness_allows("harness")
  assert !witness_allows("runner")
  assert !witness_allows("gleam")
  assert witness_allows("gleam/erlang/process")
  []
}

/// `failure`'s two types and their six constructors, none internal.
fn failure_vocabulary() -> List(Item) {
  [
    Item(
      "Failure",
      ["Refused", "GuardOverlap", "Defect", "BrokenInvariant"],
      False,
    ),
    Item("Cause", ["NoRuleHeld", "EffectFailed"], False),
  ]
}

/// `answers`'s five types and their six constructors, none internal.
fn answers_vocabulary() -> List(Item) {
  [
    Item("Answer", ["Batch", "NoPocketAvailable"], False),
    Item("BatchRecord", ["BatchRecord"], False),
    Item("TickReport", ["TickReport"], False),
    Item("LeaseView", ["LeaseView"], False),
    Item("HeldView", ["HeldView"], False),
  ]
}

fn failure_items() -> List(String) {
  let found =
    coverage.read(failure_path)
    |> result.map(fn(source) { by_name(items(source)) })
  assert found == Ok(by_name(failure_vocabulary()))
  []
}

fn failure_imports() -> List(String) {
  assert result.map(coverage.read(failure_path), imported_modules) == Ok([])
  []
}

fn answers_items() -> List(String) {
  let found =
    coverage.read(answers_path)
    |> result.map(fn(source) { by_name(items(source)) })
  assert found == Ok(by_name(answers_vocabulary()))
  []
}

fn answers_imports() -> List(String) {
  let found =
    coverage.read(answers_path)
    |> result.map(fn(source) {
      list.sort(imported_modules(source), string.compare)
    })
  assert found == Ok(["pachinko_double/model", "pachinko_double/time"])
  []
}

/// The witness reads at least one import line, and every module they name
/// is one it may import.
fn witness_imports() -> List(String) {
  let source = coverage.read(witness_path)
  assert result.map(source, fn(text) { imported_modules(text) != [] })
    == Ok(True)
  assert result.map(source, witness_faults) == Ok([])
  []
}

/// A module's items split by their mark, each half sorted by name: first the
/// unmarked items, which are interface, then the marked ones.
pub fn by_mark(source: String) -> #(List(Item), List(Item)) {
  let #(marked, unmarked) =
    list.partition(items(source), fn(item) { item.internal })
  #(by_name(unmarked), by_name(marked))
}

/// The hand-made module's two halves: only `Seam` is marked.
fn module_halves() -> #(List(Item), List(Item)) {
  #(by_name(list.filter(module_items(), fn(item) { !item.internal })), [
    Item("Seam", ["Seam"], True),
  ])
}

/// A marked item that loses its mark moves to the unmarked half, and an
/// unmarked item that gains one moves to the marked half.
fn marks() -> List(String) {
  assert by_mark(string.join(module(), "\n")) == module_halves()
  let lost = list.filter(module(), fn(line) { line != "@internal" })
  assert by_mark(string.join(lost, "\n"))
    == #(
      by_name(
        list.map(module_items(), fn(item) { Item(..item, internal: False) }),
      ),
      [],
    )
  let gained =
    list.flat_map(module(), fn(line) {
      case line {
        "pub fn start(limit: Int) -> Int {" -> ["@internal", line]
        _ -> [line]
      }
    })
  let #(unmarked, marked) = module_halves()
  assert by_mark(string.join(gained, "\n"))
    == #(
      list.filter(unmarked, fn(item) { item.name != "start" }),
      by_name([Item("start", [], True), ..marked]),
    )
  []
}

/// Whether a source holds a line that starts with
/// `import pachinko_double/internal/`.
pub fn imports_internal(source: String) -> Bool {
  list.any(string.split(source, "\n"), fn(line) {
    string.starts_with(line, "import pachinko_double/internal/")
  })
}

/// An indented import, a comment and a module whose path only starts alike
/// are no internal import.
fn reads_internal_imports() -> List(String) {
  assert imports_internal(
    "import gleam/list\nimport pachinko_double/internal/core\n",
  )
  assert imports_internal(
    "import pachinko_double/internal/rules.{type Table}\n",
  )
  assert !imports_internal(string.join(import_lines(), "\n"))
  assert !imports_internal(
    "  import pachinko_double/internal/core\n// import pachinko_double/internal/core\nimport pachinko_double/internals\n",
  )
  []
}

/// The lines of a source that import a module under an alias: an import line
/// of the package that holds ` as `, or the line `} as ...` that closes an
/// import's name list.
pub fn aliased_imports(source: String) -> List(String) {
  list.filter(string.split(source, "\n"), fn(line) {
    {
      string.starts_with(line, "import pachinko_double")
      && string.contains(line, " as ")
    }
    || string.starts_with(line, "} as ")
  })
}

/// An alias on one line and an alias after a name list over several lines
/// are each found; a plain import, a name list's closing `}` and an indented
/// alias are not.
fn reads_aliased_imports() -> List(String) {
  assert aliased_imports(string.join(import_lines(), "\n"))
    == ["import pachinko_double/core as engine"]
  assert aliased_imports(
      "import pachinko_double/internal/rules.{\n  type Table,\n} as table\nimport pachinko_double/model\nimport pachinko_double/time.{\n  type Duration,\n}\n  import pachinko_double/domain as d\n",
    )
    == ["} as table"]
  assert aliased_imports("import pachinko_double/internal/core as engine\n")
    == ["import pachinko_double/internal/core as engine"]
  []
}

/// The eight vocabulary modules, by name.
fn vocabulary_modules() -> List(String) {
  [
    "answers", "config", "failure", "inspection", "invariants", "model",
    "refusals", "time",
  ]
}

/// The eleven modules under `internal/`: the engine and the specification's
/// rules.
fn internal_modules() -> List(String) {
  [
    "consuming", "core", "decay", "domain", "expiry", "identity", "lease_end",
    "publish", "pull", "rules", "settle",
  ]
}

fn vocabulary_path(module: String) -> String {
  vocabulary_directory <> "/" <> module <> ".gleam"
}

fn vocabulary_top_level() -> List(String) {
  let entries =
    simplifile.read_directory(vocabulary_directory)
    |> result.map(listing)
  assert entries
    == Ok(list.sort(
      ["internal", ..list.map(vocabulary_modules(), fn(m) { m <> ".gleam" })],
      string.compare,
    ))
  assert simplifile.is_directory(internal_directory) == Ok(True)
  []
}

fn internal_top_level() -> List(String) {
  let entries =
    simplifile.read_directory(internal_directory)
    |> result.map(listing)
  assert entries
    == Ok(list.map(internal_modules(), fn(module) { module <> ".gleam" }))
  []
}

/// The lines of a source that are a mark: `@internal`, however indented.
/// They are read as lines, not as items, so that a mark counts wherever it
/// sits.
pub fn mark_lines(source: String) -> List(String) {
  list.filter(string.split(source, "\n"), fn(line) {
    string.trim(line) == "@internal"
  })
}

/// The hand-made module's one mark is found; an indented mark is found too,
/// and a comment that holds the word is not.
fn reads_mark_lines() -> List(String) {
  assert mark_lines(string.join(module(), "\n")) == ["@internal"]
  assert mark_lines(
      "pub fn run() -> Nil {\n  Nil\n}\n  @internal\n// @internal\n/// @internal\n",
    )
    == ["  @internal"]
  assert mark_lines("pub fn run() -> Nil {\n  Nil\n}\n") == []
  []
}

/// Every module under `src/` that carries a mark, with its count: twelve in
/// `model`, one in `inspection`, two in `invariants`. The root, `config`,
/// `time`, `refusals`, `failure`, `answers` and every module under
/// `internal/` carry none.
fn marks_per_module() -> List(String) {
  assert result.map(source_modules(), fn(modules) {
      list.filter_map(modules, fn(found) {
        case list.length(mark_lines(found.1)) {
          0 -> Error(Nil)
          count -> Ok(#(found.0, count))
        }
      })
    })
    == Ok([
      #(vocabulary_path("inspection"), 1),
      #(vocabulary_path("invariants"), 2),
      #(vocabulary_path("model"), 12),
    ])
  []
}

/// A vocabulary module, from `gleam-double/`: its unmarked items, which are
/// interface, and its marked items, which are not.
type Vocabulary {
  Vocabulary(path: String, interface: List(Item), marked: List(Item))
}

/// The module holds exactly its interface unmarked and exactly its marked
/// items marked. The comparison ignores order.
fn holds(vocabulary: Vocabulary) -> List(String) {
  assert result.map(coverage.read(vocabulary.path), by_mark)
    == Ok(#(by_name(vocabulary.interface), by_name(vocabulary.marked)))
  []
}

fn plain(name: String) -> Item {
  Item(name, [], False)
}

fn marked(name: String) -> Item {
  Item(name, [], True)
}

fn config_vocabulary() -> Vocabulary {
  Vocabulary(
    vocabulary_path("config"),
    [
      Item("Settings", ["Settings"], False),
      plain("Config"),
      plain("defaults"),
      plain("default"),
      plain("new"),
      plain("settings"),
    ],
    [],
  )
}

fn time_vocabulary() -> Vocabulary {
  Vocabulary(
    vocabulary_path("time"),
    list.map(
      ["Timestamp", "Duration", "millisecond", "second", "minute", "hour"],
      plain,
    ),
    [],
  )
}

/// `model`'s nineteen unmarked items, the entities and the lookups over a
/// snapshot's lists, and its twelve marked ones.
fn model_vocabulary() -> Vocabulary {
  Vocabulary(
    vocabulary_path("model"),
    [
      Item("Queue", ["Queue"], False),
      Item("Publisher", ["Publisher"], False),
      Item("Consumer", ["Consumer"], False),
      Item(
        "PocketStatus",
        ["PocketQueued", "PocketLeased", "PocketIdle", "PocketParked"],
        False,
      ),
      Item("Pocket", ["Pocket"], False),
      Item("Mark", ["Unsettled", "Acked", "Nacked"], False),
      Item("RecordStatus", ["RecordQueued", "RecordHeld"], False),
      Item("Record", ["Record"], False),
      Item("Reason", ["Superseded", "Stale"], False),
      Item("SupersededRecord", ["SupersededRecord"], False),
      Item("DeadLetter", ["DeadLetter"], False),
      Item(
        "LeaseStatus",
        ["LeaseActive", "LeaseReleased", "LeaseExpired"],
        False,
      ),
      Item("Lease", ["Lease"], False),
      plain("pocket"),
      plain("record"),
      plain("lease"),
      plain("records_of"),
      plain("held"),
      plain("active"),
    ],
    [
      Item("State", ["State"], True),
      marked("new"),
      Item("HeldRecord", ["HeldRecord"], True),
      marked("held_under"),
      marked("replace_record"),
      marked("replace_pocket"),
      marked("replace_lease"),
      marked("queued"),
      marked("oldest_first"),
      marked("ending"),
      marked("leases_of_pocket"),
      marked("leases_of_consumer"),
    ],
  )
}

fn refusals_vocabulary() -> Vocabulary {
  Vocabulary(
    vocabulary_path("refusals"),
    [
      Item(
        "Refusal",
        [
          "ConfigRefused", "PublisherAlreadyJoined", "ConsumerAlreadyJoined",
          "UnknownPublisher", "EqualRankRefused", "UnknownConsumer",
          "PullRefused", "LeaseFenced", "RecordNotHeld", "MarkConflict",
          "ReleaseRefused",
        ],
        False,
      ),
    ],
    [],
  )
}

fn inspection_vocabulary() -> Vocabulary {
  Vocabulary(
    vocabulary_path("inspection"),
    [
      Item("Snapshot", ["Snapshot"], False),
      plain("empty"),
      Item("PocketView", ["PocketView"], False),
      Item("ConsumerView", ["ConsumerView"], False),
      Item("Views", ["Views"], False),
      plain("views"),
    ],
    [marked("snapshot")],
  )
}

fn invariants_vocabulary() -> Vocabulary {
  Vocabulary(vocabulary_path("invariants"), [plain("check")], [
    Item("Invariant", ["Invariant"], True),
    marked("registry"),
  ])
}

fn vocabulary_imports() -> List(String) {
  let importers =
    list.filter_map(vocabulary_modules(), fn(module) {
      let path = vocabulary_path(module)
      case coverage.read(path) {
        Ok(source) ->
          case imports_internal(source) {
            True -> Ok(path)
            False -> Error(Nil)
          }
        Error(fault) -> Ok(fault)
      }
    })
  assert importers == []
  []
}

fn no_aliased_imports() -> List(String) {
  let aliased = case simplifile.get_files(source_directory) {
    Ok(files) ->
      list.filter(files, fn(file) { string.ends_with(file, ".gleam") })
      |> list.sort(string.compare)
      |> list.flat_map(fn(file) {
        case coverage.read(file) {
          Ok(source) ->
            list.map(aliased_imports(source), fn(line) { file <> ": " <> line })
          Error(fault) -> [fault]
        }
      })
    Error(error) -> [
      "cannot list "
      <> source_directory
      <> ": "
      <> simplifile.describe_error(error),
    ]
  }
  assert aliased == []
  []
}

/// The names that are `pub` only for the tests: each is a `pub fn` under
/// `internal/` or marked `@internal`, and no other module under `src/` names
/// it. The guide's table under `### What is pub only for the tests` lists the
/// same names.
fn test_only() -> List(String) {
  [
    "core.run", "core.run_at", "core.run_due", "core.registry", "rules.dispatch",
    "invariants.registry",
  ]
}

/// The heading of the guide's table of the names that are `pub` only for the
/// tests.
pub const test_only_heading = "### What is pub only for the tests"

/// The first cell of each body row of the guide's table of test-only names.
/// A row is a line after `test_only_heading`, up to the next heading, that
/// starts with `|` once its leading spaces are dropped; the first two rows
/// are the header and the separator. A cell is read between its first two
/// `|`, trimmed, and without its backticks when it is one name in backticks,
/// so that a row spelled otherwise still yields its cell and the names
/// differ.
pub fn table_names(guide: String) -> List(String) {
  string.split(guide, "\n")
  |> list.drop_while(fn(line) { line != test_only_heading })
  |> list.drop(1)
  |> list.take_while(fn(line) { !string.starts_with(line, "#") })
  |> list.filter(fn(line) { string.starts_with(string.trim_start(line), "|") })
  |> list.drop(2)
  |> list.map(first_cell)
}

fn first_cell(row: String) -> String {
  let cell = case string.split(row, "|") {
    [_, cell, ..] -> string.trim(cell)
    _ -> row
  }
  case cell {
    "`" <> rest ->
      case string.split_once(rest, "`") {
        Ok(#(name, "")) -> name
        _ -> cell
      }
    _ -> cell
  }
}

/// A guide made by hand: the table under its heading, with a line of prose
/// and a header row above the rows, and a table under the next heading.
fn guide_lines() -> List(String) {
  [
    "## Where to look", "", test_only_heading, "", "Prose about `core.run`.", "",
    "| Name | What the tests use it for |", "| --- | --- |",
    "| `core.run` | Running one table |", "| `rules.dispatch` | Asking a row |",
    "", "## The next section", "", "| `core.apply` | Not this table |",
  ]
}

fn reads_table() -> List(String) {
  assert table_names(string.join(guide_lines(), "\n"))
    == ["core.run", "rules.dispatch"]
  let added =
    list.flat_map(guide_lines(), fn(line) {
      case line {
        "| `rules.dispatch` | Asking a row |" -> [line, "| `core.seam` | New |"]
        _ -> [line]
      }
    })
  assert table_names(string.join(added, "\n"))
    == ["core.run", "rules.dispatch", "core.seam"]
  let removed =
    list.filter(guide_lines(), fn(line) {
      line != "| `rules.dispatch` | Asking a row |"
    })
  assert table_names(string.join(removed, "\n")) == ["core.run"]
  assert table_names("| `core.run` | No heading above it |\n") == []
  let spelled =
    list.flat_map(guide_lines(), fn(line) {
      case line {
        "| `rules.dispatch` | Asking a row |" -> [
          line,
          " | `core.indented` | A space before the row |",
          "|`core.tight`|No space in the cells|",
          "| core.bare | No backticks |",
        ]
        _ -> [line]
      }
    })
  assert table_names(string.join(spelled, "\n"))
    == [
      "core.run", "rules.dispatch", "core.indented", "core.tight", "core.bare",
    ]
  []
}

/// One public function or constant of a module: its name, whether it is a
/// function, and whether it is marked `@internal`.
pub type Value {
  Value(name: String, function: Bool, internal: Bool)
}

/// A module's public functions and constants, in the file's order: the lines
/// that start at column 0 with `pub fn` or `pub const`.
pub fn values(source: String) -> List(Value) {
  string.split(source, "\n")
  |> list.fold(#("", []), fn(state, line) {
    let #(previous, found) = state
    let value = case line {
      "pub fn " <> rest -> [Value(word(rest), True, previous == "@internal")]
      "pub const " <> rest -> [
        Value(word(rest), False, previous == "@internal"),
      ]
      _ -> []
    }
    #(line, list.append(found, value))
  })
  |> fn(state) { state.1 }
}

fn reads_values() -> List(String) {
  assert values(string.join(module(), "\n"))
    == [Value("timeout", False, False), Value("start", True, False)]
  assert values(
      "@internal\npub fn seam() -> Nil {\n  Nil\n}\n@internal\npub const limit = 1\n  pub fn indented() -> Nil\n",
    )
    == [Value("seam", True, True), Value("limit", False, True)]
  []
}

/// Whether a source names `module.function`, where `module` is a module's
/// last segment: a line, up to its first `//`, holds the text
/// `module.function` with no letter, digit or `_` directly before `module`
/// or directly after `function`, or the source's import of the module lists
/// `function` unqualified, outside a comment.
pub fn names(source: String, module: String, function: String) -> Bool {
  let lines = string.split(source, "\n")
  let needle = module <> "." <> function
  list.any(lines, fn(line) {
    list.any(
      list.window_by_2(string.split(uncommented(line), needle)),
      fn(pair) { !ends_with_word(pair.0) && !starts_with_word(pair.1) },
    )
  })
  || list.contains(unqualified(lines, module, []), function)
}

fn is_word(grapheme: String) -> Bool {
  string.contains(
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_",
    grapheme,
  )
}

fn ends_with_word(text: String) -> Bool {
  result.map(string.last(text), is_word) == Ok(True)
}

fn starts_with_word(text: String) -> Bool {
  result.map(string.first(text), is_word) == Ok(True)
}

/// The names that the imports of a module, by its last segment, list
/// unqualified, leaving out the types.
fn unqualified(
  lines: List(String),
  module: String,
  found: List(String),
) -> List(String) {
  case lines {
    [] -> found
    [line, ..rest] ->
      case line {
        "import " <> path ->
          case segment(module_name(path)) == module {
            True -> {
              let #(text, rest) = statement(line, rest)
              unqualified(rest, module, list.append(found, listed(text)))
            }
            False -> unqualified(rest, module, found)
          }
        _ -> unqualified(rest, module, found)
      }
  }
}

/// An import statement's text, without its comments, joined over its lines
/// when it opens a name list it does not close, and the lines after it.
fn statement(line: String, rest: List(String)) -> #(String, List(String)) {
  let opening = uncommented(line)
  case string.contains(opening, "{") && !string.contains(opening, "}") {
    False -> #(opening, rest)
    True -> {
      let #(inside, after) =
        list.split_while(rest, fn(next) { !string.starts_with(next, "}") })
      case after {
        [close, ..after] -> #(
          string.join(
            [opening, ..list.map(list.append(inside, [close]), uncommented)],
            " ",
          ),
          after,
        )
        [] -> #(
          string.join([opening, ..list.map(inside, uncommented)], " "),
          [],
        )
      }
    }
  }
}

/// A line up to its first `//`, which opens its comment. An import holds no
/// string, so there it always does; on any other line a string that holds
/// `//` cuts the line short there too, and no module under `src/` holds one.
fn uncommented(line: String) -> String {
  case string.split_once(line, "//") {
    Ok(#(code, _)) -> code
    Error(Nil) -> line
  }
}

/// The names an import statement's text lists between `{` and `}`, leaving
/// out the types.
fn listed(text: String) -> List(String) {
  case string.split_once(text, "{") {
    Error(Nil) -> []
    Ok(#(_, after)) -> {
      let inside = case string.split_once(after, "}") {
        Ok(#(inside, _)) -> inside
        Error(Nil) -> after
      }
      string.split(inside, ",")
      |> list.map(string.trim)
      |> list.filter(fn(entry) {
        entry != "" && !string.starts_with(entry, "type ")
      })
      |> list.map(word)
    }
  }
}

/// The last segment of a module's path or of a file's path, without
/// `.gleam`.
fn segment(path: String) -> String {
  let last = result.unwrap(list.last(string.split(path, "/")), "")
  case string.ends_with(last, ".gleam") {
    True -> string.drop_end(last, 6)
    False -> last
  }
}

fn reads_names() -> List(String) {
  assert names("  core.run(table)\n", "core", "run")
  assert names("let ran = #(core.run, 1)\n", "core", "run")
  assert !names("  core.run_at(table, now)\n", "core", "run")
  assert !names("  score.run(table)\n", "core", "run")
  assert !names("  // core.run(table)\n/// core.run\n", "core", "run")
  assert !names("  |> list.filter(f) // was model.queued\n", "model", "queued")
  assert names("  model.queued(state) // the queue\n", "model", "queued")
  assert !names("  rules.run(table)\n", "core", "run")
  assert names("import pachinko_double/internal/core.{run}\n", "core", "run")
  assert names(
    "import pachinko_double/internal/core.{\n  type Ran, run_at,\n  run,\n}\n",
    "core",
    "run",
  )
  assert !names(
    "import pachinko_double/internal/core.{\n  type run, run_at,\n}\n",
    "core",
    "run",
  )
  assert !names("import pachinko_double/internal/rules.{run}\n", "core", "run")
  assert names(
    "import pachinko_double/internal/core.{\n  type Trigger,\n  // Read the rule tables.\n  registry,\n}\n",
    "core",
    "registry",
  )
  assert names(
    "import pachinko_double/internal/core.{ // the engine, }\n  run_at, // the clock, then }\n  run,\n}\n",
    "core",
    "run",
  )
  assert !names(
    "import pachinko_double/internal/core.{\n  type Ran,\n  // run,\n  run_at,\n}\n",
    "core",
    "run",
  )
  []
}

/// A module's path is under `internal/`.
fn under_internal(path: String) -> Bool {
  string.contains(path, "/internal/")
}

/// The other modules that name `module.function`, by path.
fn users(
  modules: List(#(String, String)),
  path: String,
  module: String,
  function: String,
) -> List(String) {
  list.filter_map(modules, fn(other) {
    case other.0 != path && names(other.1, module, function) {
      True -> Ok(other.0)
      False -> Error(Nil)
    }
  })
}

/// The faults of a list of test-only names against modules, each a path and
/// its source. A listed name must be a `pub fn` of its module, under
/// `internal/` or marked, that no other module names. Every other `pub fn`
/// and `pub const` under `internal/` or marked must be named by some other
/// module.
pub fn test_only_faults(
  modules: List(#(String, String)),
  listed: List(String),
) -> List(String) {
  let listed_faults =
    list.flat_map(listed, fn(name) {
      case string.split_once(name, ".") {
        Error(Nil) -> [name <> " is not written module.function"]
        Ok(#(module, function)) ->
          case list.find(modules, fn(found) { segment(found.0) == module }) {
            Error(Nil) -> [name <> ": no module is named " <> module]
            Ok(#(path, source)) -> {
              let defined =
                list.find(values(source), fn(value) {
                  value.function && value.name == function
                })
              let placed = case defined {
                Ok(value) ->
                  case value.internal || under_internal(path) {
                    True -> []
                    False -> [
                      name <> " is neither under internal/ nor marked",
                    ]
                  }
                Error(Nil) -> [name <> " is no pub fn of " <> path]
              }
              list.append(
                placed,
                list.map(users(modules, path, module, function), fn(user) {
                  name <> " is named by " <> user
                }),
              )
            }
          }
      }
    })
  let unlisted =
    list.flat_map(modules, fn(found) {
      let #(path, source) = found
      let module = segment(path)
      values(source)
      |> list.filter(fn(value) { value.internal || under_internal(path) })
      |> list.filter(fn(value) {
        !list.contains(listed, module <> "." <> value.name)
      })
      |> list.filter_map(fn(value) {
        case users(modules, path, module, value.name) {
          [] ->
            Ok(
              module
              <> "."
              <> value.name
              <> " is pub only for the tests, and the list does not hold it",
            )
          _ -> Error(Nil)
        }
      })
    })
  list.append(listed_faults, unlisted)
}

/// Modules made by hand: `core.run` is the one test-only name. The root
/// names `core.apply` qualified and `model.replace` unqualified across
/// several lines; `model.lookup` is unmarked, so no module need name it.
fn scan_modules() -> List(#(String, String)) {
  [
    #(
      "src/pachinko_double.gleam",
      "import pachinko_double/internal/core\nimport pachinko_double/model.{\n  type Pocket,\n  replace,\n}\n\npub fn start() -> Nil {\n  core.apply()\n}\n",
    ),
    #(
      "src/pachinko_double/internal/core.gleam",
      "pub fn run() -> Nil {\n  Nil\n}\n\npub fn apply() -> Nil {\n  Nil\n}\n",
    ),
    #(
      "src/pachinko_double/model.gleam",
      "@internal\npub fn replace() -> Nil {\n  Nil\n}\n\npub fn lookup() -> Nil {\n  Nil\n}\n",
    ),
  ]
}

/// Changes one module's source in `scan_modules()`.
fn scan_with(
  path: String,
  change: fn(String) -> String,
) -> List(#(String, String)) {
  list.map(scan_modules(), fn(found) {
    case found.0 == path {
      True -> #(found.0, change(found.1))
      False -> found
    }
  })
}

/// The scan's result changes when a second module starts to name a listed
/// function, when a new function under `internal/` is named by no module,
/// when a marked function loses its last user, when the list drops a name
/// and when it holds a name that is not test-only.
fn scan() -> List(String) {
  assert test_only_faults(scan_modules(), ["core.run"]) == []
  let named =
    scan_with("src/pachinko_double.gleam", fn(source) {
      source <> "\npub fn go() -> Nil {\n  core.run()\n}\n"
    })
  assert test_only_faults(named, ["core.run"])
    == ["core.run is named by src/pachinko_double.gleam"]
  let added =
    scan_with("src/pachinko_double/internal/core.gleam", fn(source) {
      source <> "\npub fn seam() -> Nil {\n  Nil\n}\n"
    })
  assert test_only_faults(added, ["core.run"])
    == ["core.seam is pub only for the tests, and the list does not hold it"]
  let unused =
    scan_with("src/pachinko_double.gleam", fn(source) {
      string.replace(source, "  replace,\n", "")
    })
  assert test_only_faults(unused, ["core.run"])
    == [
      "model.replace is pub only for the tests, and the list does not hold it",
    ]
  assert test_only_faults(scan_modules(), [])
    == ["core.run is pub only for the tests, and the list does not hold it"]
  assert test_only_faults(scan_modules(), ["core.run", "core.apply"])
    == ["core.apply is named by src/pachinko_double.gleam"]
  assert test_only_faults(scan_modules(), ["core.run", "model.lookup"])
    == ["model.lookup is neither under internal/ nor marked"]
  assert test_only_faults(scan_modules(), ["core.run", "core.gone"])
    == ["core.gone is no pub fn of src/pachinko_double/internal/core.gleam"]
  []
}

/// The guide, from `gleam-double/`.
pub const guide_path = "README.md"

fn guide_table() -> List(String) {
  assert result.map(coverage.read(guide_path), fn(guide) {
      list.sort(table_names(guide), string.compare)
    })
    == Ok(list.sort(test_only(), string.compare))
  []
}

/// Every module under `src/`, by path, with its source.
fn source_modules() -> Result(List(#(String, String)), String) {
  case simplifile.get_files(source_directory) {
    Ok(files) ->
      list.filter(files, fn(file) { string.ends_with(file, ".gleam") })
      |> list.sort(string.compare)
      |> list.try_map(fn(file) {
        coverage.read(file) |> result.map(fn(source) { #(file, source) })
      })
    Error(error) ->
      Error(
        "cannot list "
        <> source_directory
        <> ": "
        <> simplifile.describe_error(error),
      )
  }
}

fn test_only_names() -> List(String) {
  assert result.map(source_modules(), test_only_faults(_, test_only()))
    == Ok([])
  []
}

/// The test directory and its one directory, which holds the test support,
/// from `gleam-double/`.
pub const test_directory = "test"

pub const support_directory = "test/support"

/// The three support modules: the coverage check, the fakes and the runner.
fn support_modules() -> List(String) {
  ["coverage.gleam", "harness.gleam", "runner.gleam"]
}

/// The names of a listing's entries that are not test modules, sorted. Each
/// entry is a name and whether it is a file; a test module is a file whose
/// name ends in `_test.gleam`. At the top level of `test/` that is only
/// `support`.
pub fn not_tests(entries: List(#(String, Bool))) -> List(String) {
  list.filter_map(entries, fn(entry) {
    let #(name, file) = entry
    case file && string.ends_with(name, "_test.gleam") {
      True -> Error(Nil)
      False -> Ok(name)
    }
  })
  |> list.sort(string.compare)
}

/// A directory's entries, each with whether it is a file.
fn kinds(directory: String, names: List(String)) -> List(#(String, Bool)) {
  list.map(names, fn(name) {
    #(name, simplifile.is_file(directory <> "/" <> name) == Ok(True))
  })
}

/// A directory's listing, sorted, so that two listings compare with `==`. It
/// leaves out the entries whose names start with `.`, such as `.DS_Store` or
/// an editor's swap file, which a clean tree may hold.
pub fn listing(entries: List(String)) -> List(String) {
  list.filter(entries, fn(name) { !string.starts_with(name, ".") })
  |> list.sort(string.compare)
}

/// A top level of test modules and `support` leaves only `support`; one that
/// holds `helpers.gleam`, a directory named like a test module, or no
/// `support`, leaves something else.
fn reads_test_top_level() -> List(String) {
  assert not_tests([
      #("support", False),
      #("a_test.gleam", True),
      #("pachinko_double_test.gleam", True),
    ])
    == ["support"]
  assert not_tests([
      #("a_test.gleam", True),
      #("support", False),
      #("helpers.gleam", True),
    ])
    == ["helpers.gleam", "support"]
  assert not_tests([
      #("a_test.gleam", True),
      #("support", False),
      #("extra_test.gleam", False),
    ])
    == ["extra_test.gleam", "support"]
  assert not_tests([#("a_test.gleam", True)]) != ["support"]
  []
}

/// The three support modules in any order are the support listing; a fourth
/// module, or one missing, is not.
fn reads_support_listing() -> List(String) {
  assert listing(["runner.gleam", "coverage.gleam", "harness.gleam"])
    == support_modules()
  assert listing([
      "runner.gleam", "coverage.gleam", "extra.gleam", "harness.gleam",
    ])
    != support_modules()
  assert listing(["coverage.gleam", "harness.gleam"]) != support_modules()
  []
}

/// A listing that also holds `.DS_Store` and an editor's swap file is still
/// the support listing, and a top level that holds them still leaves only
/// `support`. A name with a dot past its start stays.
fn reads_hidden_entries() -> List(String) {
  assert listing([
      "runner.gleam", ".DS_Store", "coverage.gleam", ".foo.gleam.swp",
      "harness.gleam",
    ])
    == support_modules()
  assert not_tests(
      list.map(listing([".DS_Store", "support", ".a_test.gleam.swp"]), fn(name) {
        #(name, name != "support")
      }),
    )
    == ["support"]
  assert listing([".DS_Store", "notes.swp", "a.DS_Store", "helpers.gleam"])
    == ["a.DS_Store", "helpers.gleam", "notes.swp"]
  []
}

fn test_top_level() -> List(String) {
  assert result.map(simplifile.read_directory(test_directory), fn(names) {
      not_tests(kinds(test_directory, listing(names)))
    })
    == Ok(["support"])
  assert simplifile.is_directory(support_directory) == Ok(True)
  []
}

fn support_top_level() -> List(String) {
  assert result.map(simplifile.read_directory(support_directory), listing)
    == Ok(support_modules())
  []
}
