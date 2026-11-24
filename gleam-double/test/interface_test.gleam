//// The pin on the package's interface. `items` reads a module's source and
//// returns its public items, each with its constructors and its `@internal`
//// mark; `imports_effects` says whether a module imports `gleam/erlang` or
//// `gleam/otp`; `imported_modules` names the module each import line takes.
//// All three are pure, so the controls run them on text made by hand. The
//// assertions read the tree from `gleam-double/`, where the run starts: the
//// root's items are exactly the seventeen of the interface, the root is the
//// only module under `src/` that imports an effect, and `src/` holds at its
//// top level only the root and its directory. The two vocabulary modules of
//// the replies, `failure` and `answers`, hold exactly the reply types;
//// `failure` imports nothing, and `answers` imports only `model` and `time`.
//// The witness, the walkthrough in `test/guide_test.gleam`, imports no engine
//// module and no rule module.
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
//// - an import line starts with `import `, and the module it names is the
////   text after that, up to the first space or `.`.
////
//// The text a control reads is a list of lines, so that no line of this file
//// other than its own imports starts with `import`.

import coverage
import gleam/list
import gleam/result
import gleam/string
import runner.{type Test, Test}
import simplifile

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
    Test("the root holds exactly the interface's items", [], root_items),
    Test(
      "the root is the only module under src/ that imports an effect",
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
/// `simplifile`, the support modules `harness` and `runner`, the root, or
/// one of the eight vocabulary modules.
pub fn witness_allows(module: String) -> Bool {
  string.starts_with(module, "gleam/")
  || list.contains(
    [
      "simplifile", "harness", "runner", "pachinko_double",
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
            case imports_effects(source) {
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
    |> result.map(list.sort(_, string.compare))
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
    "import gleam/option.{type Option, None}", "import harness.{type Double}",
    "import pachinko_double", "import pachinko_double.{type Reply, Outcome}",
    "import pachinko_double/core", "import pachinko_double/core.{TickReport}",
    "import pachinko_double/core as engine", "import pachinko_double/model.{",
    "  type Mark,", "}", "  import pachinko_double/domain",
    "// import pachinko_double/rules", "import runner.{type Test, Test}",
    "import simplifile",
  ]
}

fn reads_imports() -> List(String) {
  assert imported_modules(string.join(import_lines(), "\n"))
    == [
      "gleam/list", "gleam/option", "harness", "pachinko_double",
      "pachinko_double", "pachinko_double/core", "pachinko_double/core",
      "pachinko_double/core", "pachinko_double/model", "runner", "simplifile",
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
