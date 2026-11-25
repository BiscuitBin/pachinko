---
id: QM03
title: "The verify recipe and its required CI job"
status: todo
depends_on: [QM02]
parallel_with: []
branch: ticket/qm03-verify-recipe-and-ci
estimated_size: M
---

# QM03: The verify recipe and its required CI job

## Context

After QM02 the repository holds `quint/pachinko.qnt` (created by QM02): the module
`pachinko` with three rules, six invariants, the ghosts, `guard_<Rule>`, `effect_<Rule>`
and `witness_<Rule>` for each rule, `FiredMatchesGuards`, `Canary`, and the instance
`pachinko_walk`. `just check-quint` (created by QM02) typechecks it inside `just check`.
QM02 also ran, by hand, the loop that ruling 4 of `.scratch/quint-model/README.md` calls
"verified": `Canary` violated, a bounded `just quint verify` that passes, one witness and
one mutant. Those runs are quoted in QM02's hand-back notes and nothing repeats them.

What is missing is the recipe that repeats them for every instance, witness and mutant,
and the CI job that makes a red run block a merge. Ruling 2 fixes both: `verify-quint`
runs locally and in its own CI job, and that job joins `needs` of the one required
`check` job from the day it lands. Every later model ticket (QM04 to QM09, QM13) adds
rules, witnesses and mutants, and shared rule 6 makes `just verify-quint` their evidence.
So this ticket decides what a green run means for the rest of the series.

A naive recipe is wrong in five ways. Each was read in the pinned CLI's source, under
`tools/quint/node_modules/@informalsystems/quint/dist/src/`, and none was run for this
ticket unless a line says so:

- **The exit status says little.** `outputResult` (`cliCommands.js`, from line 638) exits
  0 on any right result and 1 on any left one. A syntax error, a type error, an unknown
  name, a counterexample, a deadlock and a witness that is not reached are all left
  results. `handleVerificationFailure` (`apalache.js`, from line 129) turns Apalache's
  `Deadlock` into `reached a deadlock` with a trace, and `processApalacheResult`
  (`cliReporting.js`, from line 230) reports anything with a trace as `[violation]`. A
  mutant "caught" by exit status 1 may be a typo.
- **A pass can check nothing.** `getInvariants` (`cliHelpers.js`, from line 117) checks
  the names given with `--invariant` or `--invariants`; without one it falls back to the
  definitions the main module declares with the `invariant` qualifier
  (`declaredProperties`, line 91), and it drops the literal `true`. With neither, no
  `q::inv` is compiled (`cliCommands.js`, line 493) and the run passes. The model
  declares no `invariant` definition (README, "Properties are named apart from
  everything else"), so a `verify` that names no property checks nothing.
- **A witness needs the right flag.** One `--observation` stops the run when reached and
  fails when it is not (`verify.js`, `verifyWithApalacheBackend`, from line 176).
  `--search-observations` only prints a warning for an observation it does not reach
  (`searchObservationsOnceVerified`, from line 133). An invariant violation outranks a
  reached observation (`apalacheStoppedBy`, from line 269), so a witness run that also
  checks a false property reports a violation.
- **The server that answers may not be this worktree's.** `connect` (`apalache.js`, from
  line 490) first tries a server on the endpoint, whose default is `localhost:8822`
  (`cli.js`, line 376), and uses whatever answers. Decision 0021,
  `docs/decisions/0021-project-managed-quint-cli.md`, says the guard "cannot see which
  Apalache such a server runs". AP01's hand-back notes in
  `.scratch/apalache/AP01-java-and-verify.md` (Departures, item 4, and Open points,
  "Port 8822") record what was run, with the port free: at `--verbosity=4` a run's log
  holds Quint's own spawn lines. With a server already there it would hold
  `Connecting with existing Apalache server` in their place, which is read and not run.
- **A mutant is a patch with context lines.** A context line for a blank line of the
  model is one space, and every context line starts one column to the right of the
  model's own indentation. `.editorconfig` sets `trim_trailing_whitespace = true` and
  `indent_size = 2` for every file, `editorconfig-checker` reads every text file
  (`.pre-commit-config.yaml`, `types: [text]`), and the `trailing-whitespace` fixer in
  `.pre-commit-fix.yaml` would rewrite a patch under `just fix`.

Read these:

- `.scratch/quint-model/README.md` in full. "Design fixed before QM01", both blocks;
  "Names every ticket uses"; "Claims the tickets check", rows 1 and 7 to 12; shared rules
  5, 6, 10 and 11.
- QM02's ticket, `QM02-tracer-claim-and-release.md` in `.scratch/quint-model/` (the
  README's index names it): its hand-back notes, for the by-hand commands, their output
  and the maintainer's rulings on its design points.
- `quint/pachinko.qnt` (created by QM02) and the decision 0022 record QM02 created under
  `docs/decisions/`. QM02 takes another number if 0022 was taken, so find the record by
  its key: `rg -l '^canonical_for: \[decision_quint_model\]' docs/decisions` prints its
  path, and exactly one. Read "0022" in this ticket as that record's number.
- `docs/specs/pachinko.allium`: `PullClaimsFrontPocket`, `PullFindsNoPocket`,
  `ReleaseRequeuesPocket`, and the invariants `OneActiveLeasePerPocket` and
  `OneActiveLeasePerConsumer`, each with the comments above it.
- `Justfile`: the `QUINT_HOME` comment, `install-quint`, `install-apalache`, `quint`,
  `check-clean`, and how `fix` calls another recipe. `scripts/install-apalache.sh`: the
  `home` function, which refuses a run that does not start at the repository's root, and
  the `--check` mode.
- `.github/workflows/ci.yml`: the comment over `check`, the `loadgen` job and its
  comment, and each job's `timeout-minutes`. `.github/actions/setup/action.yml`.
- `docs/decisions/0021-project-managed-quint-cli.md`, whole.
- `.scratch/apalache/README.md`, "Out of scope for both"; AP01's and AP02's hand-back
  notes on port 8822 (`.scratch/apalache/AP01-java-and-verify.md`,
  `.scratch/apalache/AP02-checksummed-apalache.md`).
- `.editorconfig`, `.pre-commit-config.yaml`, `.pre-commit-fix.yaml`, `_typos.toml`.
- The CLI source named above: `verify.js`, `cli.js`, `cliHelpers.js`, `cliCommands.js`,
  `cliReporting.js`, `apalache.js`.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

`just verify-quint` verifies the model and fails closed. For every instance module of
`quint/pachinko.qnt` it shows `Canary` violated, then every property passing at the
bound `quint/verify.txt` states, each property named on the command line. It reaches
every rule's witness, and it applies every mutant under `quint/mutants/` to a scratch
copy and shows the mutant caught by the property it names. Zero instances, a list that
disagrees with the model, a witness not reached, a patch that does not apply, a mutant
that does not typecheck, survives, deadlocks or is caught only by another property, a
run that errors, a listener on port 8822 before or after a run, and a run whose log
does not show Quint starting its own server each fail it. Point 4 says what that leaves
open. It leaves the worktree unchanged and reaches no network.

The CI job `quint` installs the CLI, Apalache and the evaluator, runs the recipe and
`just check-clean`, and is in `needs` of `check` in the same change. Decision 0021 is
amended, decision 0022 says what a green run shows and does not show, and the handbook
names the recipe wherever it lists recipes, gates and CI jobs. The first two mutants are
committed, and the witnesses of QM02's three rules run in the recipe.

### Owns

| Kind | Names |
| --- | --- |
| Rules | none |
| Invariants | none |
| Guarantees | none |
| Partial or extended, not counted | none. The recipe runs the three witnesses and every property QM02 stated; it adds no definition to the model |

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them at the **Stop** in step 4,
with checkpoint 1's findings in hand, and the hand-back records the ruling. After that
the builder does not reopen them. Points 1 to 4 are the four rows of the README's
"Design points a ticket settles" that name QM03; the rest are choices those four force.
The later tickets are written against the recommended answers.

1. **How a witness is run** (README row). Recommended: one run per witness,
   `--no-invariant --observation=witness_<Rule>`. The run passes only if it exits 0 and
   its log holds `[observed] Observed witness_<Rule>, which stopped the run`
   (`cliReporting.js`, line 115). The comma after the name keeps one rule's name from
   matching as the prefix of another's. `--no-invariant` is there because a violated
   invariant outranks an observation. The README forbids the `invariant` qualifier in
   the model, so the flag changes nothing while that holds, and keeps a witness run
   meaning the same if a definition gains the qualifier by mistake. What the line
   shows: the CLI looks for the witness in every state of the trace Apalache returned
   and does not read why the run stopped (`verify.js`, lines 269 to 317), so a trace
   returned for a deadlock gives the same line and exit status. The witness is reached
   either way, since the trace is one the model has. What the line does not show is
   that the model has no deadlock. The pass, which runs first on the same instance at
   the same bound, is the run that would report one, if Apalache reports deadlocks at
   all under the configuration Quint builds: claim F settles that. No probe hands a
   witness run a deadlock's trace, so this branch stays read and not run (Open points,
   "A witness on a deadlock's trace"). Alternative: a
   negated invariant, `--invariant='not(witness_<Rule>)'`, that must be violated; it
   goes through the violation path, so it needs the same evidence as a mutant and prints
   the witness's trace as a counterexample. Rejected: `--search-observations`, which
   warns and passes.
2. **What counts as a mutant caught by its named property** (README row). Recommended,
   all of these, in this order:
   1. The patch has exactly one `# instance:` line and one `# property:` line before its
      first `---` line (point 5), the instance is one the model has, and the property is
      any one the recipe checks on it (shared rule 6), never `Canary`.
   2. The patch names `a/quint/pachinko.qnt` and `b/quint/pachinko.qnt` and no other
      file, applies with exact context to a scratch copy, and the copy differs from the
      model (`cmp` exits 1).
   3. `just quint typecheck` passes on the copy.
   4. `just quint verify`, naming only that property, on that instance at its bound,
      exits 1, and its log holds all three of: `[violation] Found an issue`
      (`cliReporting.js`, line 246); `error: found a counterexample` and not
      `error: reached a deadlock` (`apalache.js`, lines 144 and 146, printed by
      `outputResult`); and exactly one line made of two spaces, the cross mark U+274C,
      a space and a name, that name being the property (`cliReporting.js`, line 171).
      The script writes the cross mark as `printf '\342\235\214'` and matches the whole
      line with `grep -Fx`, so the script itself stays ASCII.
   5. The property is in the list the instance's pass named, and that pass, on the
      unchanged model at the same bound, printed `[ok] No violation found` (line 234).
      This is what shared rule 6 asks for: the pass checks the conjunction that holds
      the property, so it shows the property to hold unchanged, and no further run per
      property is made. The pass runs before any mutant.
   6. The patched copy defines the named property as the model does. The helper of
      point 6 compares that one definition in the two parsed files, ids and source
      positions aside, and a patch that changes it fails with
      `changes the property it names`. It is a static check: it runs with items 1 to
      3, after the copy typechecks and before any run (point 7). Without this, a patch
      that replaces the property's body by `false` is "caught" and no action was
      mutated. Write the comparison over a named set of definitions, here a set of
      one, so that QM13, which compares every definition but one for its weakened
      invariant, adds a case and no second comparer. The check does
      not follow the definitions the property calls: a mutant may change a definition
      an action and a property share, and checkpoint 2, and each later ticket's
      review, asks whether a mutant is caught for the Allium's reason. Alternative: no
      such check, with the question left to review. QM13's weakening edits an
      invariant on purpose; it is a run of another kind, and QM13 states its rule.

   The set of mutants is the directory: the recipe runs every `*.patch` under
   `mutants/`, and nothing lists the patches that must be there. A patch deleted by a
   later change leaves the recipe green with one mutant fewer. The counts in the first
   and last lines, which every hand-back quotes, and the diff under review are what
   show it. Alternative: one line per mutant in `quint/verify.txt`, compared with the
   directory, which every later ticket that adds a mutant would then edit.

   A mutant whose run prints `[ok]` survives. Any other outcome is an error, not a
   catch. A mutant that another property would catch survives too: its run names one
   property, so the recipe cannot tell the two apart, and its message says so.
   Alternatives for the evidence in 4: the `--out` JSON, whose `status` is `violation`
   for a deadlock as well, since `pickOutputStage` (`cliReporting.js`, line 292) keeps
   neither the message nor the violated name, and `--out` sets the verbosity to 0
   (`deriveVerbosity`, `cliHelpers.js`, line 252), which also hides the server lines of
   point 4; or `--out-itf` and a second evaluation of the property on the trace's last
   state, which does not depend on the reporter and costs another tool. Alternative for
   5: a run of the property alone on the unchanged model, which shows nothing the pass
   does not and costs one full-depth run per property.
3. **`quint/verify.txt`, `--max-steps` and the timeout** (README row).
   - The file holds comment lines starting with `#`, blank lines, and one line per
     instance: the module's name, one space, `max-steps=<n>`. Today that is the single
     line `pachinko_walk max-steps=10`. The recipe reads it with `sed` and never sources
     it. It fails on a malformed line, a name given twice, an instance of the model with
     no line, and a line whose module the model does not have. A line of a kind it does
     not know fails by name (point 5, last paragraph). Alternative: `key=value`
     pairs as `tools/quint/apalache.txt` has, which cannot carry a second field per
     instance.
   - `--max-steps` for `pachinko_walk`: 10, the CLI's default (`cli.js`, line 353),
     always written out. Step 2 measures 6, 8, 10 and 12. If 10 does not fit point 8's
     head-room, the answer is the largest measured bound that does, and never less than
     the deepest witness's depth plus two. Every run on an instance uses its bound.
     Alternative: the smallest bound that reaches every witness, which is cheap and
     leaves a later ticket nothing to raise from.
   - `timeout-minutes: 20` for the job, the most any job in `ci.yml` has today (`rust`,
     `coverage` and `image`).
4. **What the recipe shows about the server that answers** (the README row asks how it
   "makes sure the Apalache server that answers is the one it started"; it cannot make
   sure, and "What stays open" below says what is left). Recommended:
   - Before anything else, and again before every run, a probe connects to port 8822
     on each address `localhost` resolves to, and on `127.0.0.1` and `::1`. Any answer
     fails the recipe, naming the listener if `lsof` is present. The probe is pixi's
     Python, as `scripts/install-apalache.sh` already uses it, so it does not depend on
     `lsof`.
   - Every run is at `--verbosity=4`, and its log must hold
     `No running Apalache server found, launching...` and
     `Started Apalache server on pid=`, and must not hold
     `Connecting with existing Apalache server` (`apalache.js`, lines 495, 499 and 515).
     That is the rule for a run that connects once, as every run of this ticket does.
     A run that connects more than once in one invocation logs the connecting line
     when it meets the server it started itself: `--inductive-invariant` connects once
     for each of its phases (`verify.js`, lines 203 to 227). The ticket that adds such
     a run states that row's rule: the launching line and the started line come first,
     once each, and every connecting line comes after them.
   - Every run has `grpc_proxy`, `https_proxy` and `http_proxy` unset. QM01 recommends
     that the `quint` recipe unset them, with their upper-case forms, for every command
     (QM01, "The proxy variables"), and step 1 quotes `just --show quint` to see. If
     the recipe does not, QM01's ruling was the alternative, and the script unsets the
     three itself with `env -u` before `just`. The CLI's gRPC client reads them, in
     that order and in lower case only, and sends a connection to `localhost:8822`
     through the proxy unless `no_grpc_proxy` or `no_proxy` names the host
     (`tools/quint/node_modules/@grpc/grpc-js/build/src/http_proxy.js`, lines 35 to 55
     and 160 to 190). The probe's raw socket ignores them, so with one set the probe and
     Quint would not be talking to the same port.
   - After every run the recipe waits up to ten seconds for the port to be free, and
     fails if it is not.
   - It never passes `--server-endpoint`, `--apalache-config`, `--backend` or
     `--random-transitions`, and it never kills a process. A server left by an
     interrupted run is found by the next run's first probe; the message prints the
     listener and says to stop it by hand, because the recipe cannot tell its own
     leftover from another worktree's live run.
   - The pages say the recipe is never run in two worktrees at once.

   What stays open: Quint's gRPC connection is not tied to the process it spawned
   (AP01's Open points). The CLI logs the pid as soon as `spawn` returns and only then
   retries the connection to the endpoint (`apalache.js`, lines 490 to 531). A listener
   that binds the port after the probe and before the spawned server does can answer
   that connection, and if it is gone again before the next probe, every line and
   every probe above is satisfied. So the recipe shows that Quint found no server and
   started one, and that the port was free before and after; it does not show which
   process answered. Alternative: a machine-wide lock directory held for the whole
   recipe, which closes the race between two worktrees and leaves a stale lock after a
   kill. Rejected: a `--server-endpoint` on a free port, which shared rule 10 forbids.
5. **A mutant's header.** Lines before the first `---` line are the header; `git apply`
   skips them (claim E). Three are required, in this form, and a line starting `#` with
   any other key fails:

   ```text
   # mutant: a claim leaves the pocket queued, so a second consumer can claim it
   # instance: pachinko_walk
   # property: OneActiveLeasePerPocket
   ```

   The patch is cut with `diff -u` at three lines of context, with the labels
   `a/quint/pachinko.qnt` and `b/quint/pachinko.qnt`. The recipe copies the model to
   `<tmp>/<mutant>/quint/pachinko.qnt` and runs `git -C <tmp>/<mutant> apply` there,
   giving the patch by its absolute path, since `-C` changes where a relative one is
   looked for. `git apply` takes no fuzz. Alternative: `patch -p1` with zero fuzz, whose
   flag differs between GNU and BSD `patch` (not read).

   Both formats grow by kinds. QM13 adds a second kind of line to `quint/verify.txt`
   and a second kind of patch header. So the reader of each tells a line's kind first
   and fails on a kind it does not know, naming it, which makes a new kind one case
   more and never a line read as something else; and point 2's check that a patch
   leaves its named property alone applies to the `# property:` kind only.
6. **How the recipe reads the model.** `just quint parse --out=<tmp>/model.json` (offline,
   and outside the guard's Apalache check), read by one helper,
   `scripts/verify-quint-model.py`, on pixi's Python. The README fixes the naming this
   rests on (shared rule 3, and "Properties are named apart from everything else"): a
   property is a `val` with no parameter whose name starts with an upper-case letter, or
   `effect_<Rule>`; every other definition starts with a lower-case letter; nothing uses
   the `invariant` qualifier. So the helper classifies by name, and fails on anything
   that breaks the naming:
   - an instance is a module named `pachinko_<bound>`; any other module beside `pachinko`
     fails, and so does a second `.qnt` file in the directory;
   - an instance holds its import of `pachinko`, which sets the constants, and beyond
     that only definitions that start with a lower-case letter and have no `guard_`,
     `effect_` or `witness_` prefix. A property, or a qualified definition, in an
     instance fails: the lists below are read from `pachinko` alone, so the recipe
     would never name it;
   - the rules are the variants of the sum type `Rule`, which holds the rules modelled
     so far and no other (QM02, "`Rule` grows with the model");
   - each rule has `guard_<Rule>`, `effect_<Rule>` and `witness_<Rule>`, and a definition
     with one of those prefixes whose suffix is not a rule fails;
   - `Canary`, `FiredMatchesGuards` and `UnansweredStepChangesNothing` exist;
   - a property is each `effect_<Rule>` and every top-level `val` of `pachinko` that
     takes no parameter and starts with an upper-case letter, except `Canary` and the
     variants of a sum type, which the parsed model lists as definitions too (claim C);
   - a type declaration is not a definition for these rules. Quint requires a type's
     name to start with an upper-case letter, so `Rule`, `Step` and the state's types
     all do, and the parsed model gives each the kind `typedef`
     (`parsing/ToIrListener.js`, lines 293 to 322), apart from the constructors it
     generates for a sum type's variants (lines 375 to 425);
   - a definition with the `invariant` or `observation` qualifier fails, and so does
     any other definition that starts with an upper-case letter.

   The recipe prints the list for each instance, so a hand-back can quote it, and names
   it on the command line. A name the model lacks is refused by Quint as an argument
   error, which the recipe reports as an error. Alternative: the lists written by hand
   in `quint/verify.txt` and compared with a `sed` over the source, which reads layout
   where this reads the parsed model. A model the helper refuses is a **Stop**, not an
   edit to the model.
7. **The runs, in order.** The first port probe, then the static checks, so a busy port
   or a broken list or patch fails in seconds: the model's lists, `quint/verify.txt`,
   every patch's header, apply, `cmp` and typecheck, and the comparison of point 2,
   item 6. Then, for each instance: the canary
   run, the pass, each witness. Then each mutant. Every `verify` has this shape, with
   the model's path first because `--invariants` is an array option that takes every
   word after it (`cli.js`, line 334; claim A):

   ```sh
   FORCE_COLOR=0 just quint verify <file> --main=<instance> --init=init --step=step \
     --max-steps=<n> --verbosity=4 <property flags>
   ```

   | Run | Property flags | It passes when |
   | --- | --- | --- |
   | Canary | `--invariants <every property> Canary` | exit 1, and the evidence of point 2.4 with `Canary` as the one name |
   | Pass | `--invariants <every property>` | exit 0 and `[ok] No violation found` |
   | Witness | `--no-invariant --observation=witness_<Rule>` | exit 0 and the `[observed]` line of point 1 |
   | Mutant | `--invariants <its property>`, on the patched copy | exit 1, and the evidence of point 2.4 with its property as the one name |

   The canary run is the pass's own command with `Canary` appended, so it also shows
   that the last name of the list is read. A second cross-mark line in it is a real
   property violated on the unchanged model, and the recipe says which. The canary run
   stops at the first step, so it does not show that the pass itself carries the list:
   a pass built without its flags checks nothing and prints `[ok]`. Step 6's control
   "a property that fails late" shows it, with a property that holds until after the
   canary run has stopped. Output goes to
   a log per run under the temporary directory; on a failure the recipe prints the
   log's last 40 lines. The shape allows what later tickets add. A second instance,
   such as QM09's `pachinko_zero_ttl`, is one more line in `quint/verify.txt`, and gets
   its own canary, pass and every witness. A further kind of run, such as QM13's
   inductive one, is one more row of this table, with its own server rule where it
   connects more than once (point 4).
8. **The time budget.** The recipe prints each run's seconds and a last line with the
   total. The timeout covers the whole job: the setup action, the three installs, every
   run and `check-clean`. This ticket controls only the recipe, so its own limit is on
   that step: QM03 lands only if `just verify-quint` takes at most 5 minutes in CI, and
   the hand-back quotes the setup's and the installs' durations beside it. A later
   ticket whose additions bring the whole job over 15 of the 20 minutes stops and asks;
   it does not shrink a domain, lower a bound, drop a run or raise the timeout on its
   own (ruling 4, shared rule 5). Per instance the cost is one full-depth pass; the
   canary, the witnesses and the mutants each stop at the first trace that answers
   them. The levers are the maintainer's: a larger `timeout-minutes`, a cache for the
   installs, or a matrix over instances.
9. **The recipe's interface.** `just verify-quint` takes one optional argument, the
   model directory, which defaults to `quint` and holds `pachinko.qnt`, `verify.txt` and
   `mutants/`. The controls of step 6 pass a copy under `ai_tmp/qm03/`, so no control
   edits `quint/`. The first and last output lines name the directory, and only a run on
   `quint` is evidence. The script refuses to start anywhere but the repository's root,
   as `scripts/install-apalache.sh` does, and never changes directory: `just quint` is
   refused elsewhere. Scratch copies live in a directory from
   `mktemp -d "${TMPDIR:-/tmp}/verify-quint.XXXXXX"`, which the script refuses if it is
   inside a Git worktree (claim E) and removes in a trap through `"${tmp:?}"`.
   Apalache's `_apalache-out/` lands in the repository's root, which `.gitignore`
   already covers.
10. **What the recipe prints.** Every line starts `verify-quint:`. A failure starts
    `verify-quint: FAILED:` and exits 1. The fragments below are what Verification
    matches, so they are part of the proposal. A row's first fragment is on the
    `verify-quint: FAILED:` line itself, which is where Verification looks for it, so
    that a name echoed in some earlier diagnostic does not count; what a row lists
    after that may follow on later lines:

    | Failure | The message holds |
    | --- | --- |
    | No model file | `pachinko.qnt is missing` |
    | No instance module | `no instance module` |
    | A second `.qnt` file, or a module that is neither `pachinko` nor an instance | the file's name, or the module's |
    | An instance without a bound | `is not in` and the path of `verify.txt` |
    | A bound without an instance | `which the model does not have` |
    | A rule without a witness, or a witness without a rule | `has no witness`, or `names no rule` |
    | A definition that breaks the README's naming | the definition's name, and the rule it breaks |
    | A property, or a qualified definition, in an instance | `in an instance`, and the definition's name |
    | Something answers on port 8822 | `port 8822 is in use`, the listener, and the advice of point 4 |
    | A run that did not start its own server | `did not start its own Apalache server` |
    | The canary holds | `Canary was not violated` |
    | A property fails on the unchanged model | `violated on the unchanged model`, and each cross-mark line |
    | A witness is not reached | `was not reached within <n> steps` |
    | A patch does not apply | `does not apply`, the output of `git apply`, and: re-cut it against the current model, keeping its mutation and its property (shared rule 11) |
    | A mutant edits the property it names | `changes the property it names` |
    | A mutant does not typecheck | `does not typecheck: an error, not a catch` |
    | A mutant survives | `survives: <property> held for <n> steps`, and: if another property catches it, the patch names the wrong one |
    | A mutant's run errors or deadlocks | `not a counterexample to <property>` |

    A green run has this shape. The names are the model's and the figures are only
    an illustration:

    ```text
    verify-quint: quint: 1 instance, 11 properties, 3 witnesses, 2 mutants
    verify-quint: pachinko_walk: max-steps 10; properties: FiredMatchesGuards ...
    verify-quint: pachinko_walk: Canary violated (4 s)
    verify-quint: pachinko_walk: 11 properties hold for 10 steps (31 s)
    verify-quint: pachinko_walk: witness_PullClaimsFrontPocket reached (5 s)
    verify-quint: qm03-claim-leaves-pocket-queued: caught by OneActiveLeasePerPocket (6 s)
    verify-quint: not mutation-tested: FiredMatchesGuards ...
    verify-quint: quint: verified in 62 s: 1 instance, 11 properties, 3 witnesses, 2 mutants, 7 runs
    ```

    The `not mutation-tested:` line lists the properties no mutant names, for shared
    rule 6's report.
11. **The patches and the hook gate.** `.editorconfig` gains a section
    `[quint/mutants/*.patch]` with `indent_size = unset` and
    `trim_trailing_whitespace = false`, as `[*.md]` has, and the `trailing-whitespace`
    hook in `.pre-commit-fix.yaml` excludes `^quint/mutants/.*\.patch$` beside
    `^tickets/.*\.md$`. `typos` reads the patches like any text and nothing is excluded
    for it: a mutant's words obey it as the model's do. `check-quint` globs `.qnt` files,
    so no patch reaches it. Alternative: patches with no context lines, which have no
    trailing space and no way to notice that the model moved under them.
12. **The first two mutants.** Each is one edit to an action, never to a `guard_` or
    `effect_` definition, so that the transcribed properties stay what QM02 wrote:
    - `quint/mutants/qm03-claim-leaves-pocket-queued.patch`: the claim no longer sets
      `pocket.status = leased`, which `PullClaimsFrontPocket` ensures. A second consumer
      claims the same pocket. Property: `OneActiveLeasePerPocket`.
    - `quint/mutants/qm03-claim-ignores-held-lease.patch`: the claim no longer requires
      `not exists Lease{consumer: consumer, status: active}`. A consumer claims a second
      pocket. Property: `OneActiveLeasePerConsumer`.

    The slugs and the exact edits are settled at checkpoint 1 against QM02's text. QM02's
    by-hand mutant may be one of them.
13. **The CI job.** After `loadgen` in `.github/workflows/ci.yml`, with a comment that
    says it is a CI job and not a gate in `just check`, and why:

    ```yaml
    quint:
      name: quint
      runs-on: ubuntu-latest
      timeout-minutes: 20
      steps:
        - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
          with:
            persist-credentials: false
        - uses: ./.github/actions/setup
          with:
            cache-key: quint
        - run: just install-quint
        - run: just install-apalache
        - run: just install-quint-evaluator
        - run: just verify-quint
        - run: just check-clean
    ```

    The checkout line is copied from the other jobs as they are at the base, never from
    this ticket. `check`'s `needs` gains `quint` in the same commit. The job has no `if:`,
    because `check` fails on a skipped job. No cache holds `.quint/` or
    `tools/quint/node_modules/` yet: each install checks what it downloads, and point 8's
    measurement says whether a cache is worth its key.

## Claims this ticket checks

Numbered rows are the README's. Lettered rows are this ticket's own. Each is settled by
running the pinned CLI through `just quint` and quoting the output. An answer that
differs from the one stated is a design change: stop and bring it to the maintainer.

| # | Claim | Check |
| --- | --- | --- |
| 1 | A `verify` that passes does not start the Rust evaluator; one that reports a violation does, and so does an `--observation` that is reached. So the witness path of point 1 needs the evaluator QM01 installs (read in `verify.js` and `rust/invariantsReporter.js`, not run) | step 2: the logs of a pass, a violation and a reached witness; step 7: the whole recipe with the external network denied |
| 7 | The properties a `verify` checks are the ones named on its command line; with none named it falls back to the definitions declared with the `invariant` qualifier, and the model declares none. A run can therefore pass having checked nothing, so a pass means something only after `Canary` was violated through the same instance (read in `cliHelpers.js` lines 91 and 117 and `cliCommands.js` line 493, not run) | step 2: a `verify` with no property flag, and one with `--no-invariant`, each pass on a copy whose every property is false; step 6: the control "the canary holds" |
| 8 | Quint tries a server on `localhost:8822` before it starts its own (read in `apalache.js` from line 490; AP01 ran the half with the port free at `5b74af3`, and no record shows the connecting line printed) | step 2: the three log lines of point 4, on a run with the port free, and the connecting line matched in `apalache.js` itself, since shared rule 10 forbids the run that would print it; step 6: the control "port busy" |
| 9 | One `--observation` passes when reached and fails when not; `--search-observations` only warns, and several `--observation` flags pass when one is reached (read in `verify.js` from lines 133 and 176, not run). `--search-observations` searches the definitions declared with the `observation` qualifier and nothing named with `--observation` (`cliCommands.js`, lines 497 to 498; `verify.js`, line 134) | step 2: a reached witness; an unreachable one; two `--observation` flags, one reached and one not; and `--search-observations` on a copy that declares the unreachable one with the `observation` qualifier. Each with its exit status and result line |
| 10 | A syntax error, a type error and a counterexample all exit 1, and a deadlock is reported as a violation (read in `cliCommands.js` from line 638 and `apalache.js` from line 129, not run) | step 2: the four runs and their exit statuses; step 6: the controls "ill-typed" and "deadlock" |
| 11 | The output names the violated invariant only when the reporter can evaluate the counterexample (read in `cliReporting.js` from line 261, not run) | step 2: the cross-mark line of a caught mutant, quoted byte for byte with `od -c`; its absence on the deadlock run |
| 12 | The whole of `verify-quint` finishes inside the job's timeout, with the head-room of point 8 (not read) | step 7, locally; step 13, in CI, with the run's URL |
| A | `--invariants` takes every word after it, the input file included, so the file goes first (read in `cli.js` line 334 and the comment at line 60, not run) | step 2: the same run with the file last, quoted |
| B | `--no-invariant` with `--observation` is accepted and leaves the observation as the only thing checked (read in `cliHelpers.js` from line 171, not run) | step 2: the witness run on a copy whose `Canary` is declared `invariant` |
| C | `just quint parse --out` writes JSON whose modules list each definition's kind, qualifier and name, each sum type's variants, and each instance's module; a variant also appears as a definition (read in `parsing/ToIrListener.js` lines 379 to 423, not run) | step 2: the helper's output on the model, beside the model's text |
| D | With `FORCE_COLOR=0` and the output sent to a file, no line carries a colour code, locally and on the runner (not read) | step 2: a search for the escape byte in a log, after a positive control with `FORCE_COLOR=1`; step 13 for the runner, where the logs are gone after a green run: the evidence there is that every line the recipe matched whole was found, which a colour code inside one would have prevented, and nothing wider |
| E | `git apply`, run through `git -C` in a directory outside any worktree, skips leading header lines, applies only on exact context, and exits non-zero otherwise; inside a worktree's subdirectory it can skip a path silently (not read) | step 2: a patch that applies, one with a changed context line, and one run inside `ai_tmp/`, each with its exit status and a `cmp` |
| F | What Apalache 0.62.4 reports for a `step` that is never enabled, under the configuration `createConfig` builds, which sets no deadlock option (read in `apalache.js` from line 98; Apalache's side not read). And what a witness run prints when the trace returned for a deadlock holds a state the witness is true in: read in `verify.js`, lines 269 to 317, it is `[observed]` and exit 0 | step 2: the run, quoted; the control "deadlock" then expects that branch. Then the pass on a copy whose `step` stops being enabled after a few steps, quoted: it shows whether a deadlock fails a pass. The witness half is not probed: Quint asks Apalache for the negated observation (`verify.js`, lines 120 to 121), so a witness run meets the witness's state as an ordinary counterexample before any deadlock behind it, and no fixture hands it a deadlock's trace |
| G | The runner has what the recipe needs: `mktemp -d`, `git`, pixi's Python, and no exported bash function, which the `quint` recipe's guard refuses (`scripts/install-apalache.sh`, the `BASH_FUNC_` test) | step 13: the first CI run |
| H | A denial that blocks the external network and allows loopback lets a `verify` pass; on macOS a `sandbox-exec` profile, on Linux `unshare -rn` with `lo` brought up (not read, not run) | step 7: a loopback connection succeeds and §11's two `curl` commands fail inside the same denial |
| I | After each run the port is free again within the bounded wait (AP01 ran it on macOS at `5b74af3`; not run on Linux). The recipe waits for the port, not for the pid it read from the log: a process can outlive its listener, and nothing here shows the process gone | every run of step 5; step 13 for Linux, where a green run means every wait succeeded |
| J | A `verify` of a file outside the worktree leaves nothing in the worktree but `_apalache-out/` | step 7: a `find -cnewer` over the whole worktree, ignored directories included, after its positive control, a file rewritten with its modification time put back. `git status --short --ignored` and `just check-clean` show only Git's view, in which an ignored directory such as `.quint/` is one line whatever changes inside it |
| K | `editorconfig-checker` reports a patch's context lines, for their odd indentation and for the single space that stands for a blank line, and the `trailing-whitespace` fixer rewrites that space (read in `.editorconfig`, `.pre-commit-config.yaml` and `.pre-commit-fix.yaml`, not run) | step 5: `just lint` and `just fix` on the patches, before point 11's two edits and after |
| L | Each of QM02's three witnesses is reached on `pachinko_walk` at its bound. The seed queues two pockets for three consumers, so that `PullFindsNoPocket` can fire (README, under "What each ticket delivers") | step 2: each witness run at every measured bound, and the shallowest bound that reaches it |

## Non-goals

- No rule, invariant, guarantee, guard, effect or witness is added to the model, and
  `quint/pachinko.qnt` is not edited. If the recipe cannot read the model as QM02 landed
  it, stop and ask.
- No change to `just check`'s recipe list in `pyproject.toml`. `verify-quint` is slow
  and needs Apalache and the evaluator.
- No inductive invariant: QM13 adds that run.
- No trace fixtures. `just quint-traces`, and the regeneration-and-diff step it appends
  to `scripts/verify-quint.sh`, are QM10's.
- No second instance, which is QM09's, and no change to `pachinko_walk`'s domains or
  configuration.
- No change to the `quint` recipe's guard, to how Quint picks its endpoint, or to
  either installer.
- No pre-commit hook that runs Quint, and no `verify-quint` in
  `.github/workflows/release.yml`.
- No cache for the Quint installs in CI.
- No `CHANGELOG.md` entry. It records the crates' changes.

## Shared rules (QM01 to QM13)

1. **The Allium decides.** `docs/specs/pachinko.allium` is the source of truth for
   behaviour (decision 0002). The model is derived from it and from nothing else. Read
   each rule and invariant in the Allium at the ticket's base, never from a ticket's
   summary of it, a page or a chat. No ticket reads the Rust or the double to decide
   what the model does. A ticket may read them for names, for the walk's domains and
   configuration, and, in QM10 to QM12, to map steps and state.
2. **The model never leads.** A counterexample has one of three causes. The model
   transcribes the Allium wrongly: fix the model. The property transcribes the Allium
   wrongly: fix the property, against the Allium's text. The Allium is wrong or silent:
   stop, write the trace and the question under Open points, and ask the maintainer. Never
   weaken an invariant, narrow a domain, drop a rule or add a guard the Allium does not
   have to make a run pass. Never settle either `open question` in the Allium.
3. **Names are fixed.** The model is `quint/pachinko.qnt`, module `pachinko`. An
   instance is a module `pachinko_<bound>` at its foot; the first is `pachinko_walk`.
   The sum type `Rule` has one variant for each Allium rule the model has so far, spelt
   as the Allium spells it.
   A rule has `guard_<Rule>`, `effect_<Rule>` and `witness_<Rule>`. An Allium invariant
   or guarantee is a property under its Allium name; `FiredMatchesGuards` and
   `UnansweredStepChangesNothing` check the labels. The ghosts are `fired`, `last`,
   `answer` and `prev`. A mutant is `quint/mutants/<ticket>-<slug>.patch`. Top-level
   actions carry `WalkStep`'s names (`crates/libpachinko/tests/queue/walk.rs`) in lower
   camel case. The gate is `just check-quint`, the verify recipe is `just verify-quint`,
   and the trace fixtures live in `quint/traces/`.
4. **One action per cascade, and every step is total.** A pull with its `HoldRecord`
   steps is one action. A lease's end with its `Freed` rules is one action. A publish
   with the revival it causes is one action. A step that no rule answers changes
   nothing and leaves `fired` empty; from QM09, `advance` and a `tick` with nothing due
   move only the clock. No guard and no effect on the state reads a ghost.
5. **The domains are a ruling.** `pachinko_walk` has the walk's domains and
   configuration: 2 publishers, 3 consumers, 3 pockets, 3 record keys, ranks 0 to 3,
   pull limits -1 to 3 (-1 and 0 until QM05), `max_pull_batch` 2 and
   `max_consecutive_expiries` 2. A ticket
   that cannot verify at these inside the CI job's timeout stops and asks. It does not
   shrink a domain or drop an instance. QM03 sets each instance's `--max-steps`; a
   later ticket may raise it and never lowers it.
6. **Verified means this.** From QM03 on, `just verify-quint` shows all of it. `Canary`
   is violated on every instance. Every property passes on every instance, each named
   on the command line. Every rule the ticket owns has its witness reached. Every
   mutant the ticket lists typechecks and is caught by the one property it names, which
   may be any property the recipe checks, and which the unchanged model's pass on that
   instance shows to hold at the same bound. Each check fails closed: zero instances, a
   witness not reached, a patch that does not apply, a mutant that survives, a mutant
   caught by another property, and an error or a deadlock read as a catch are all
   failures. QM02 shows the same by hand for one witness and one mutant. An owned
   invariant that no listed mutant aims at is reported in the hand-back as checked and
   not mutation-tested.
7. **No clock before QM09.** QM02 to QM08 add no `now`, no timestamp field and no TTL.
   While the model has no clock, a specification change that only says when something
   is due changes nothing in the model, as long as the untimed lapse still covers it.
8. **What the model lacks is written down.** A behaviour the Allium allows and the
   model does not yet have is one line in the model's header comment, under "Not yet
   modelled", naming the ticket that adds it. That ticket deletes the line. A property
   that would fail only because of such a gap carries the gap as a named exception, and
   the same ticket removes the exception. A property is transcribed whole, conditions
   included, even where a condition is always true so far.
9. **Quint runs through the recipe.** Every Quint command is `just quint <args>`, run
   from the repository's root, never a bare `quint` on `PATH`. A claim about the CLI's
   behaviour is checked by running the pinned CLI and quoting its output, never from
   memory or from a page.
10. **The network is named, and the checker is the pinned one.** `just initialize`,
    `just install-quint`, `just install-apalache` and `just install-quint-evaluator`
    reach the network, and each is a named, authorised step in the ticket that runs it
    (`tickets/CONVENTIONS.md` §11, "Network"). `just check` reaches none. No step lets
    Quint download anything itself. Before any `verify`, show that nothing is listening
    on `localhost:8822`; never pass `--server-endpoint`; never run a `verify`, or any
    other `just quint` command that needs the evaluator, while `just install-apalache`
    or `just install-quint-evaluator` runs or while another worktree verifies. A `verify` talks
    to its own server over the loopback interface, so a network denial used as evidence
    for a `verify` denies the external network and allows loopback, and its negative
    controls are §11's two `curl` commands.
11. **One change carries the model and what depends on it.** A change to
    `quint/pachinko.qnt` re-cuts every mutant patch it breaks, keeping the mutation and
    the property it names, and runs `just verify-quint` once QM03 has landed that recipe.
    Once `quint/traces/` exists it also regenerates the fixtures, before any gate runs,
    because `just check-quint` then compares the model's hash, and keeps both replays
    green: the Rust test in `just test` and, once QM12 has landed, `just python-double`. A
    field or a step the model gains is either mapped in each replay or added to that
    replay's list of what it does not compare, naming the ticket that will map it. A step
    on a replay's list ends that replay of every trace that holds it, so a ticket lists a
    step only with the maintainer's answer on which ticket maps it. The patches, the
    fixtures, their generator and the replays are in the Files touched of every ticket
    that changes the model, and the hand-back lists each patch re-cut. A change to
    `docs/specs/pachinko.allium` that touches a modelled rule, invariant or guarantee
    updates the model in the same change, runs `just check-quint`, and runs
    `just verify-quint` if the recipe exists.
12. **Files.** Edit only the files the ticket lists, plus its own `status:` line and
    hand-back notes. A shared file is listed as `shared, own lines`. A change needed
    elsewhere is handed back, not made (`tickets/CONVENTIONS.md` §11). Scratch goes in
    `ai_tmp/<ticket>/` and is never committed.
13. **Evidence, and the series base.** Each quoted output is headed by the commit it ran
    at and shows its exit status. A check that passes by printing nothing follows its
    positive control (`tickets/CONVENTIONS.md` §11, "Quoted evidence names its commit"
    and "Checks fail closed"). Where §11 says `main`, read the series base the
    maintainer named: for the worktree's start, for the closing fetch and merge, and
    for the evidence quoted afterwards. Pushing and opening a pull request are
    separately authorised.

## Adversarial review

This ticket runs `tickets/CONVENTIONS.md` §14 at each checkpoint it marks: its phases,
its two lenses, its two rounds at most, its scope guard and its record. These deltas
apply, and every ticket of the series carries them word for word.

- **Reviewer.** Codex, Sol, which is `codex exec -m gpt-6.2-sol`. The maintainer named
  Sol for this series on 2026-11-25, in place of the model §14 prints. Confirm
  `codex exec --help` on the day, as §14 asks. Fall back to a fresh-context
  `claude-opus-5-5` subagent only if `codex` is not on `PATH`, and say so in the
  hand-back notes.
- **A usage limit is not a clean review.** If a Codex log ends in a usage-limit error,
  the lens is incomplete. Stop and ask the maintainer whether to wait or to use the
  fallback. Never record it as "no findings".
- **Command.** Run this from the worktree, once per lens:

  ```sh
  dir="$PWD/ai_tmp/review/<id>-<n>"
  mkdir -p "$dir"
  codex exec -m gpt-6.2-sol -s read-only -C "$PWD" \
    -o "$dir/last-<lens>.md" "<brief>" \
    < /dev/null >| "$dir/log-<lens>.txt" 2>&1 &
  pid=$!
  wait "$pid"
  echo "exit $?"
  ```

  `<id>` is the ticket id, `<n>` the checkpoint's number, and `<lens>` the lens. A
  second round uses `<lens>-2` in all three names. `< /dev/null` closes stdin; without
  it the run waits forever. `-o` holds only the last message. `>|` is for a shell with
  `noclobber`. Under `-s read-only` Codex cannot write files, so the brief tells it to
  put its complete findings in its final message, and the runner copies
  `last-<lens>.md` to `findings-<lens>.md` under a first line `Source: last-message`.
  If §14 on the ticket's base already prints a command that redirects stdin from
  `/dev/null`, use §14's, with Sol's model.
- **Completion.** A lens is complete when its process, followed by the process id the
  command captured, has exited 0 and `last-<lens>.md` is not empty. Never wait on a
  file name or a `pgrep` pattern, which can match a run in another worktree. A lens
  that exits non-zero or leaves an empty file runs again, once; a second failure stops
  the ticket.
- **Lenses.** `conformance` and `breakage`, as §14 defines them. A checkpoint marked
  "with a design lens" adds a third reviewer, `design`: does the model say what the
  Allium says, is anything the Allium allows missing without being listed under "Not
  yet modelled", and could a property hold for a reason other than the one claimed.
- **Brief.** It gives the ticket's path, `tickets/CONVENTIONS.md`,
  `docs/specs/pachinko.allium`, the checkpoint's questions, and the artefact. At a plan
  checkpoint the artefact is the plan file. At a checkpoint on a model or a recipe it
  is the files the ticket names there, with the output of the controls run so far. At
  a diff checkpoint it is `git diff "$base"...HEAD` plus the uncommitted changes.
  `base` is the commit step 1 records. The brief never gives the author's reasoning or
  the hand-back's conclusions.
- **Triage and apply.** Separate fresh subagents check each finding by a command or a
  read and write `ai_tmp/review/<id>-<n>/verdicts.md`. They apply valid fixes one
  writer at a time. Before the lead runs `just check` or `just verify-quint`, it
  confirms every writer has stopped.
- **After the last checkpoint.** A merge or a rebase onto the series base, or a fix made
  after the last checkpoint, as after a red CI run, that changes a file under the
  ticket's Files touched was not reviewed. The agent shows that diff
  at the push stop and asks whether the last checkpoint runs over it first.
- **Network.** Each checkpoint authorises its model call (§11, "Network").

`<id>` is `QM03`. Checkpoints:

- 1, in step 3: the plan, `ai_tmp/qm03/plan.md`, before any tracked file changes. The
  **Stop** for the ruling on "Design proposed" follows it, in step 4.
- 2, in step 8: the recipe and its controls, before the CI job joins `needs`.
- 3, in step 11: the diff, before the final commit.

## Files touched

QM01 and QM02 edit several of these files before this ticket's base exists. Find each
sentence with `rg` at the base; the quotations in step 10 are from `cf1f3f3`.

| Path | Class | Content |
| --- | --- | --- |
| `scripts/verify-quint.sh` | new | the runner, POSIX `sh`, executable like the other scripts, with a header comment that states points 2, 4 and 7 |
| `scripts/verify-quint-model.py` | new | the helper of point 6; no shebang, run as `python -I -B`, as `scripts/diagrams.py` is run by its recipe |
| `quint/verify.txt` | new | the bounds, with a comment naming decision 0022 and shared rule 5's "never lowers" |
| `quint/mutants/qm03-claim-leaves-pocket-queued.patch` | new | the first mutant of point 12 |
| `quint/mutants/qm03-claim-ignores-held-lease.patch` | new | the second |
| `Justfile` | shared, own lines | `verify-quint`, with a comment; the sentence "Not a gate (decision 0021)" in the `quint` recipe's comment |
| `.editorconfig` | shared, own lines | the section of point 11, with a comment |
| `.pre-commit-fix.yaml` | shared, own lines | the `trailing-whitespace` hook's `exclude` |
| `.github/workflows/ci.yml` | shared, own lines | the `quint` job and its comment; `quint` in `check`'s `needs` |
| `docs/decisions/0021-project-managed-quint-cli.md` | shared, own lines | the amendment of step 10 |
| `docs/decisions/0022-<slug>.md` | shared, own lines | created by QM02, under the path step 1 finds by its `canonical_for` key, whatever its number: the recipe, its bounds, and what a green run shows and does not show |
| `docs/reference/commands.md` | shared, own lines | a `just verify-quint` row; the list of recipes outside `just check`; the `just quint <args>` row's sentence on gates |
| `docs/reference/quality-gates.md` | shared, own lines | "Outside the gate"; "In continuous integration": the job table, the counts of jobs, the paragraph "The mapping is not one to one"; "On `main`": the jobs `check` needs |
| `docs/reference/testing.md` | shared, own lines | one sentence that names `just verify-quint` and points at decision 0022 for what it proves and does not: beside QM02's sentence on the model's checks if QM02 added one (its row was conditional, and its author found no place for it), otherwise where the page says what checks the design |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | run `just verify-quint` after a change to the model; how to cut and re-cut a mutant; never two worktrees at once |
| `docs/how-to/develop-locally.md` | shared, own lines | "Quint": the recipe, and what it needs installed |
| `docs/explanation/security-model.md` | shared, own lines | the sentence "No gate runs those commands.", and what the recipe does about a server on `localhost:8822` |
| `docs/reference/configuration.md` | shared, own lines | rows for `quint/verify.txt` and the two scripts; the `.editorconfig` and `.pre-commit-fix.yaml` rows |
| `docs/project/repository-map.md` | shared, own lines | the `scripts/` lines name the two scripts; `quint/` names `verify.txt` and `mutants/` |
| `AGENTS.md` | shared, own lines | `just verify-quint` in the same-change rule QM02 wrote |
| `.agents/skills/spec-change/SKILL.md` | shared, own lines | the same, in the skill's body; its frontmatter is frozen and its bridges are not touched |
| `.scratch/quint-model/QM03-verify-recipe-and-ci.md` | ticket | `status:`, the hand-back notes, and a correction to Verification that a control shows is needed, recorded under Departures |
| `.scratch/quint-model/README.md` | shared, own lines | QM03's Status cell; and, only if the maintainer rules against a recommended answer, the sentences that ruling changes there |
| `.scratch/quint-model/QM04-*.md` to `QM13-*.md` | shared, own lines | only if the maintainer rules against a recommended answer: the sentences that ruling changes, each listed under Design rulings |
| `tickets/T39-decide-q2-q6.md`, `tickets/T40-pocket-order-model.md`, `tickets/T42-clock-model.md`, `tickets/T43-caller-contract.md`, `tickets/T45-drop-joining.md`, `tickets/T46-no-claim-without-records.md`, `tickets/T47-start-rule-and-config-bounds.md` | shared, own lines | only if the maintainer's ruling differs from a recommended answer, and only in a ticket still `todo`: the paths and commands that its "The Quint model" paragraph and its `quint/pachinko.qnt` row take from this ticket, each listed in the hand-back |

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm03-verify-recipe-and-ci`
   from the series base the maintainer names, once QM02 has merged there, and record the
   base's full commit id as `base`. `git ls-files .scratch/quint-model/` must list the
   README and this ticket. Run `just initialize` (network, authorised). Run `just check`
   on the untouched worktree and quote its last line, headed by the commit. Then show that
   QM01 and QM02 are in the base, quoting each: `ls quint/pachinko.qnt`,
   `rg -l '^canonical_for: \[decision_quint_model\]' docs/decisions`, which must print
   one path, `just check-quint`, and `just --list` naming `install-quint-evaluator`. If
   one is missing, stop. The path `rg` prints is decision 0022's record even if QM02
   had to take another number; a file that merely starts `0022-` is not. Quote
   `just --show quint` and say whether its `env` unsets the proxy variables (point 4).
   Read QM01's and QM02's hand-back notes; where a ruling differs from the recommended
   answer this ticket assumes, list the difference in `ai_tmp/qm03/plan.md` and raise
   it at step 4.
2. **Read and probe before writing.** No tracked file changes in this step. Work in
   `ai_tmp/qm03/`, on copies of the model; each `verify` follows a probe that shows port
   8822 free, and the probe follows its own positive control (a listener on the port,
   started as this ticket's Verification starts one). Settle claims 1, 7, 8, 9, 10, 11, A,
   B, C, D, E and F as the table says, quoting each command, its output and its exit
   status. If step 1 found that the `quint` recipe does not unset the proxy variables,
   every `verify` run by hand in this ticket, here and in steps 5 to 7, runs under
   `env -u grpc_proxy -u https_proxy -u http_proxy`: the script's own unsetting covers
   only the runs it makes. Count the variants of `Rule` in the base's model: this
   ticket's text assumes three. If the count differs, a T ticket landed between QM02
   and this one: name it, and have the plan recompute the witnesses, their depths and
   the bound from the model as it is (README, "The T tickets that touch the model").
   The plan then also lists each line of point 12, of the acceptance criteria and of
   Verification that names a witness, a mutant or a count the base no longer has, with
   its replacement, and step 4 puts that list to the maintainer with the rest. Time
   the pass on
   `pachinko_walk` at `--max-steps` 6, 8, 10 and 12, and find the shallowest bound at
   which each of the three witnesses is reached (claim L). If one is
   reached at no measured bound, **stop** and bring the model's text and the runs to the
   maintainer: the recipe cannot be green, and neither a lower standard nor an edit to the
   model is this ticket's to choose. Otherwise write `ai_tmp/qm03/plan.md`:
   - the table of point 7 with the lines the runs really printed beside the lines this
     ticket expects;
   - `quint/verify.txt` in full, and the measurements behind its bound;
   - the outline of `scripts/verify-quint.sh` against points 2, 4, 7, 9 and 10, and of
     the helper against point 6, with the helper's lists for the model as it is;
   - each mutant as a diff, with the Allium's text it breaks beside it, the one
     property that must catch it, and the trace QM02's model gives;
   - each control of step 6 and how it is built;
   - the CI job, and the edit to each page in Files touched.
3. **Adversarial review** (checkpoint 1: conformance and breakage, per
   `tickets/CONVENTIONS.md` §14, over `ai_tmp/qm03/plan.md` with step 2's output).
   Questions:
   - Can the recipe print its last green line when a property of the model was never
     named on a command line, or a witness or a mutant was never run?
   - Can a mutant count as caught when its run ended in a type error, an argument error,
     a deadlock, or a violation of a property other than the one it names?
   - Can a run be answered by a server the recipe did not start, and the recipe not
     notice?
   - Does the helper's rule miss a definition of QM02's model, or take a helper, a
     constructor or a constant for a property? Does it refuse a type declaration, or
     pass a property that sits in an instance?
   - Is each expected line something step 2 saw printed, or only something read?
   - Does any step lower a bound, shrink a domain or edit `quint/pachinko.qnt`?
4. **Stop for the ruling.** **Stop** and put "Design proposed" to the maintainer, with
   the checkpoint's verdicts, step 2's measurements and any claim whose answer differed.
   Record the ruling on each of the thirteen points. If it changes a recommended
   answer, edit the sentences it changes in the README and in the later tickets, and
   list them under Design rulings.
5. **The patches under the hooks, then the recipe, green.** Cut each patch with
   `diff -u` from a copy edited under `ai_tmp/qm03/`, never by hand, add both, and save
   them under `ai_tmp/qm03/saved/`. Red first (claim K): `just lint` fails on a patch;
   quote the lines `editorconfig-checker` reports. `just fix` changes a patch that has a
   blank context line (`cmp` against the saved copy exits 1); restore it from the saved
   copy. If neither patch has such a line, say so, and show it on a scratch patch that
   does. Then add the `.editorconfig` section and the `.pre-commit-fix.yaml` exclusion,
   and show the green half: `just fix`, then `just lint`, then `cmp` exits 0 for each
   patch. Then add the helper, `quint/verify.txt`, the runner and the `Justfile` recipe.
   With the port probe green, run `just verify-quint` and quote its whole
   output and exit status, headed by the commit and "with uncommitted changes". Run
   `sh -n scripts/verify-quint.sh`, and run the recipe once under `/bin/sh` on macOS,
   which is bash 3.2: no array, no `local`, no `[[`.
6. **Red: every branch that fails closed.** Write `ai_tmp/qm03/make-controls.sh`, which
   takes a directory and builds in it each control marked "script" below, from `quint/`
   in the worktree as step 5 left it: the model, `verify.txt` and both patches, none of
   them committed yet. Verification runs the same script after step 12's commit, when
   that tree is the committed one. For each row run `just verify-quint <copy>`, and
   quote the failing line and the exit status. `quint/` is never edited.

   The recipe's static checks run first, so a control fails for its own reason only if
   everything before its branch still passes on its copy. A control that edits the
   model does so on lines no patch has in a hunk, the drifted patch apart. A control
   aimed at a witness or a mutant leaves every property true, so the canary run and the
   pass are green before its branch is reached. A control that trips an earlier check
   all the same does not pass: its row's message is missing from the log, and that is
   a control to rebuild, not a result.

   | Control | Built how | The recipe must print |
   | --- | --- | --- |
   | Empty directory | `mkdir` | `pachinko.qnt is missing` |
   | No instance | the model cut at `module pachinko_walk` | `no instance module` |
   | A second file | a copy of the model beside it, as `other.qnt` | `other.qnt` |
   | A stray module | `module qm03_stray` appended to the model | `qm03_stray` |
   | Unlisted instance | `verify.txt` without its `pachinko_walk` line | `is not in` |
   | Stale bound | `verify.txt` with a line for `pachinko_none` | `which the model does not have` |
   | Malformed bound | `max-steps=ten` | a message naming the line |
   | Port busy | a listener on 8822, on `quint` itself | `port 8822 is in use` |
   | The canary holds | script: `Canary`'s body replaced by `true` | `Canary was not violated` |
   | A new property | script: `val ScratchFalse = false` added to `pachinko` | `violated on the unchanged model`, naming `ScratchFalse` |
   | A property that fails late | script: `val ScratchLate` added to `pachinko`, true at the start and after any one step and false after two, such as "fewer than two leases were ever granted" | `Canary violated`, then `violated on the unchanged model`, naming `ScratchLate`: the pass reported it, not the canary run |
   | A property in an instance | script: `val ScratchInstance = false` added to `pachinko_walk` | `in an instance`, naming `ScratchInstance` |
   | A rule without a witness | script: one `witness_` definition removed | `has no witness` |
   | A witness without a rule | script: `val witness_Nothing = true` added | `names no rule` |
   | A definition that breaks the naming | script: `invariant ScratchQualified = true` added | `ScratchQualified` |
   | An unreachable witness | script: `witness_ReleaseRequeuesPocket`'s body replaced by `false`, which leaves every guard, action and property as it was | `witness_ReleaseRequeuesPocket was not reached` |
   | A patch that does not apply | a hunk whose removed line the model lacks | `does not apply` |
   | A drifted patch | script: one context line of the first mutant changed in the model | `does not apply`, and the re-cut advice |
   | A patch without a property | the first mutant minus its `# property:` line | a message naming the header |
   | A patch naming no property of the model | `# property: NoSuchProperty` | a message naming it |
   | A patch on another file | a hunk for `quint/verify.txt` | a message naming the file |
   | An ill-typed mutant | a module with a type error appended | `does not typecheck` |
   | A survivor | a comment appended | `survives` |
   | A mutant caught by another property | the first mutant renamed to a property it leaves alone | `survives` |
   | A mutant that edits its property | script: the first mutant with one more hunk, which replaces the body of the property it names by `false` | `changes the property it names` |
   | A deadlock | script: a mutant whose `step` is never enabled | `not a counterexample`, or the branch claim F found; say which |

   The new-property row is also the green half of point 6: nothing but the model
   changed, and the recipe checked the new name. It is the canary run that reports
   `ScratchFalse`, so the row after it is the one that shows the pass carries the list.
   An unreachable witness is built from the witness and not from the rule's guard: with
   the guard false and the action unchanged, `FiredMatchesGuards` fails in the pass, and
   the witness branch is never reached. Two failing branches have no row,
   because neither can be produced without a race: a run whose log lacks Quint's spawn
   lines, and a port still busy ten seconds after a run. Checkpoint 2 reads both
   branches in the script instead, and Open points says so. Last, run `just fix` and
   `just lint` once more, show `cmp` exits 0 for each patch against its saved copy, and
   show that `git status --short` lists only this ticket's files.
7. **Offline, clean and timed.** With the port free, run `just verify-quint` under a
   denial that blocks the external network and allows loopback (claim H): on macOS a
   `sandbox-exec` profile, on Linux `unshare -rn` with `lo` up. Inside the same denial
   show one loopback connection succeed and §11's two `curl` commands fail, and quote
   all three exit statuses beside the recipe's last line. A sandbox that fails to launch
   is not evidence. For claim J, the positive control comes first, with a marker of its
   own. Create a file under `.quint/` and keep its times in a second file with
   `touch -r`; create `ai_tmp/qm03/marker`; wait a second; rewrite the file under
   `.quint/` and put its modification time back with `touch -r`. Show that the command
   below lists it, and that the same command with `-newer` does not; then remove the
   file. `-cnewer` reads the time the inode last changed, which a write moves and no
   `touch` can put back; `-newer` would miss a file rewritten that way. It is not in
   POSIX; GNU's and macOS's `find` both have it, and a `find` without it is a
   **stop**. The control changes `.quint/`'s own time, so the real probe starts
   afresh: create the marker again, wait a second, run the recipe, and quote this,
   which must print nothing:

   ```sh
   find . -cnewer ai_tmp/qm03/marker ! -path . \
     ! -path './.git' ! -path './.git/*' \
     ! -path './ai_tmp' ! -path './ai_tmp/*' \
     ! -path './_apalache-out' ! -path './_apalache-out/*'
   ```

   `find` reads ignored directories, which `git status --short --ignored` shows as one
   line each. A path it prints is something the recipe wrote in the worktree, so claim
   J's answer differs: record it and **stop**. Then run `just check-clean`, which shows
   Git's view. Run the recipe three times and quote each total and the per-run seconds:
   this is the local half of claim 12.
8. **Adversarial review** (checkpoint 2: conformance and breakage, per
   `tickets/CONVENTIONS.md` §14, over `scripts/verify-quint.sh`,
   `scripts/verify-quint-model.py`, `quint/verify.txt`, the two patches,
   `ai_tmp/qm03/make-controls.sh` and the output of steps 5 to 7). It runs before the CI
   job exists, because after that a wrong green blocks or admits every merge. Questions:
   - For each row of step 6, does the recipe fail for the stated reason, or for an
     earlier one that happens to print the same words?
   - Is there a failing branch of the script with no row?
   - Can a command in the script fail and the script go on: a pipeline, a command
     substitution, a `grep` whose miss is read as a pass, an unset variable in an
     `rm -rf`?
   - Would the script behave the same under dash on the runner as under bash 3.2 here?
   - Does the green output show, for each instance, the property list, the bound and
     the counts, so that a later ticket's hand-back can quote them?
   - Could the two mutants be caught for a reason other than the Allium's text they
     break?
9. **The CI job.** Add the `quint` job and put `quint` in `check`'s `needs`, in one
   commit. Run `just lint`, whose `actionlint` hook reads the workflow.
10. **Words.** Edit each page in Files touched, then run `just check-docs` and
    `just check-agents`. In `AGENTS.md` and `.agents/skills/spec-change/SKILL.md`, find
    the same-change rule QM02 wrote (shared rule 11's last sentence) and add the recipe
    to it: a change to the model, or to the Allium where it touches a modelled rule,
    invariant or guarantee, runs `just check-quint` and `just verify-quint`. Decision
    0021 says, at `cf1f3f3`:
    - under Decision, "No gate runs either command yet.";
    - under What would reopen this, "A gate that needs `quint verify`, which would
      reopen the question of Apalache's provenance only for a server the recipe did not
      start; the pinned Java it also needs is in place.";
    - under Consequences, "The guard cannot see which Apalache such a server runs".

    Amend it with a dated paragraph: `just verify-quint` now runs `verify`, CI's `quint`
    job runs the recipe and is among `check`'s `needs`, and no `just check` gate runs
    it. The reopening clause has happened. Say how the recipe answers it (point 4 as
    ruled), and what stays open: the connection is not tied to the spawned process, and
    two worktrees must not verify at once. Extend decision 0022 with the recipe, the
    file of bounds, and this: a green run shows that no trace of up to the stated steps
    from `init`, at `pachinko_walk`'s domains, violates a named property; that each
    rule's witness is reached; and that each listed mutant is caught. It does not show
    safety beyond the bound or at other domains, liveness, that a property no mutant
    aims at would notice a wrong model, or anything about the Rust or the double.
11. **Adversarial review** (checkpoint 3: conformance and breakage, per
    `tickets/CONVENTIONS.md` §14, over `git diff "$base"...HEAD` plus the uncommitted
    changes, before the final commit). Questions:
    - Does `check` fail when `quint` fails, is cancelled or is skipped, and can a pull
      request merge while `quint` has not run?
    - Does the job run anything that is not a `Justfile` recipe, or install anything
      the three install recipes do not check?
    - Could Verification pass while the recipe verified nothing, or verified a
      directory other than `quint`?
    - Does `just check` stay offline and unchanged in length, and does `check-clean`
      pass after the recipe in CI?
    - Does every page in Files touched agree on what the recipe proves, what it needs
      installed and what stays open? Does a page outside the list still say that no
      gate runs `verify`?
    - Do the patches pass `just lint` and survive `just fix`?
12. **Commit, then stop for the push.** Commit the work. Run `git fetch origin`
    (network, authorised), merge or rebase onto the series base, rerun `just check`, and
    rerun Verification and quote it, headed by the full commit id. If the merge or
    rebase changed a file under Files touched, checkpoint 3 did not see that change:
    show its diff at the stop and ask whether it is reviewed before the push (the review
    deltas, "After the last checkpoint"). **Stop**: claim 12
    needs one CI run of the `quint` job, and that needs a push and either a dispatch of
    `ci.yml` on the branch or a draft pull request. Each is separately authorised
    (`tickets/CONVENTIONS.md` §11); ask for exactly the ones needed and wait.
13. **The CI measurement.** After the authorised run, record its URL, the commit it
    tested, the `quint` job's duration and each step's, and the recipe's own last line
    from the log. Settle claims D, G and I for the runner from the same log, each as
    far as its row says a green run goes: the per-run logs are removed with the
    temporary directory, so the job's log holds the recipe's lines and no raw output.
    If the
    `just verify-quint` step took more than 5 minutes, or the job failed for a reason
    the local runs did not show,
    **stop** and bring the figures to the maintainer (ruling 4); do not lower a bound.
    A fix is a new commit that checkpoint 3 did not see: show its diff and ask whether
    it is reviewed, as after a merge. Then a new Verification and a new run, whose push
    is authorised again.
14. **Hand back.** Set `status: done` here and in the README's index, write the
    hand-back notes, and commit the record. Then run `just check` once more, on the
    record's commit: the Verification quoted in the notes ran on the commit before, and
    the hook gate has not yet read the notes. Show its last line and an empty
    `git status --short` at the stop. If it fails, fix the notes, commit and run it
    again; never amend the work's commit. **Stop** before any further push, before
    marking a pull request ready and before a merge: each needs authorisation.

## Acceptance criteria

- `just verify-quint` exits 0 on the committed `quint/`, and its quoted output shows,
  for `pachinko_walk`: the bound from `quint/verify.txt`; the list of properties, which
  holds `FiredMatchesGuards`, `UnansweredStepChangesNothing`, the three `effect_`
  properties and QM02's six invariants; `Canary` violated; the pass; the three witnesses
  reached; both mutants caught, each by the property its header names, which the pass
  named; and the last line `verify-quint: quint: verified in`.
- Each of the twenty-six controls of step 6 makes the recipe exit non-zero with the
  stated message, each quoted with its exit status. The late property is reported
  after the line `Canary violated`.
- Every `verify` the recipe ran logged that Quint started its own server, with the
  proxy variables unset by the recipe or by the script, and the port was free before
  and after each, shown by the recipe's output and by the "port busy" control. Which
  process answered is not shown (point 4).
- `just verify-quint` passes with the external network denied and loopback allowed,
  with the loopback control passing and both `curl` controls failing.
- After a run, step 7's `find` prints nothing outside `_apalache-out/`, after its
  positive control; `git status --short` lists nothing the run made; and
  `just check-clean` passes.
- Both patches are byte-identical after `just fix`, and `just lint` passes with them;
  before point 11's two edits `just lint` failed on them, as step 5 quotes.
- `.github/workflows/ci.yml` has the job `quint` with the three installs, the recipe,
  `check-clean` and `timeout-minutes: 20`, and `check`'s `needs` lists it, in one commit.
- The `quint` job passed in CI. The hand-back names the run's URL and the commit it
  tested, quotes the job's duration, each step's and the recipe's own total, shows the
  recipe's step at or under 5 minutes, and states the job's head-room against 20 minutes
  beside the three local totals.
- `pyproject.toml`'s recipe list and `quint/pachinko.qnt` are unchanged:
  `git diff "$base"...HEAD --stat` names neither.
- Decision 0021 carries the dated amendment and decision 0022 the recipe, the bounds
  and what a green run does and does not show. Every page in Files touched names the
  recipe, and `rg -n 'No gate runs' docs` finds no sentence that is no longer true.
- `AGENTS.md` and `.agents/skills/spec-change/SKILL.md` name `just verify-quint` in the
  same-change rule, and `just check-agents` passes.
- Each claim, 1 and 7 to 12 and A to L, has its outcome recorded with the output that
  settled it.
- The ruling on each of the thirteen design points is recorded, with every sentence it
  changed in the README and the later tickets.
- The three checkpoints are recorded as §14's Record asks, with the series' deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and both hold again on the
  record's commit.

## Verification

Run from a file as one `sh` script, from the repository's root, with
`.pixi/envs/default/bin` first on `PATH` so that `python` is pixi's. `set -eu` stops it
at the first command that fails. `ai_tmp/qm03/make-controls.sh` is step 6's. `first` is
the first mutant of point 12 and `other` a property it leaves alone; change both if the
ruling changed the mutants. The three witness names and the count of two catches are
those of the base this ticket was written against; if step 2's count of `Rule`
differed, the lines the plan listed are replaced as ruled at step 4, and Departures
quotes each.

```sh
set -eu
git rev-parse HEAD
first=quint/mutants/qm03-claim-leaves-pocket-queued.patch
other=UniquePocketPosition
out=ai_tmp/qm03/verify
controls=$out/controls
mkdir -p "$out"
rm -rf "${controls:?}"
mkdir "$controls"

# The port: with a listener on 8822 the recipe must refuse before any run.
python -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(30)' &
listener=$!
sleep 2
if just verify-quint >| "$out/port.log" 2>&1; then kill "$listener"; exit 1; fi
grep -F 'port 8822 is in use' "$out/port.log"
kill "$listener"
wait "$listener" || true
sleep 1

# Green, on the committed model.
just check-quint
just verify-quint >| "$out/green.log" 2>&1
grep -F 'Canary violated' "$out/green.log"
grep -F 'witness_PullClaimsFrontPocket reached' "$out/green.log"
grep -F 'witness_PullFindsNoPocket reached' "$out/green.log"
grep -F 'witness_ReleaseRequeuesPocket reached' "$out/green.log"
test "$(grep -c ': caught by ' "$out/green.log")" -eq 2
grep -F 'verify-quint: quint: verified in' "$out/green.log"

# Red. refuses <name> <text>: the recipe fails on the copy and says <text>.
refuses() {
    if just verify-quint "$controls/$1" >| "$controls/$1.log" 2>&1; then exit 1; fi
    grep '^verify-quint: FAILED:' "$controls/$1.log" >| "$controls/$1.failed"
    grep -F -- "$2" "$controls/$1.failed"
}
header() {
    printf '%s\n' '# mutant: a control, not a mutation of the queue' \
        '# instance: pachinko_walk' "# property: $1"
}
# appended <name> <line>: a copy of quint/ with a patch that appends <line> to the model.
appended() {
    cp -R quint "$controls/$1"
    cp quint/pachinko.qnt "$controls/$1.qnt"
    printf '%s\n' "$2" >> "$controls/$1.qnt"
    if diff -u quint/pachinko.qnt "$controls/$1.qnt" >| "$controls/$1.diff"; then exit 1; else test "$?" -eq 1; fi
    {
        header OneActiveLeasePerPocket
        printf '%s\n' '--- a/quint/pachinko.qnt' '+++ b/quint/pachinko.qnt'
        sed '1,2d' "$controls/$1.diff"
    } >| "$controls/$1/mutants/qm03-control.patch"
}

mkdir "$controls/empty"
refuses empty 'pachinko.qnt is missing'

cp -R quint "$controls/no-instance"
sed '/^module pachinko_walk/,$d' quint/pachinko.qnt >| "$controls/no-instance/pachinko.qnt"
refuses no-instance 'no instance module'

cp -R quint "$controls/second-file"
cp quint/pachinko.qnt "$controls/second-file/other.qnt"
refuses second-file 'other.qnt'

cp -R quint "$controls/stray-module"
printf '%s\n' 'module qm03_stray { val one = 1 }' >> "$controls/stray-module/pachinko.qnt"
refuses stray-module 'qm03_stray'

cp -R quint "$controls/unlisted"
sed '/^pachinko_walk /d' quint/verify.txt >| "$controls/unlisted/verify.txt"
refuses unlisted 'is not in'

cp -R quint "$controls/stale-bound"
printf '%s\n' 'pachinko_none max-steps=10' >> "$controls/stale-bound/verify.txt"
refuses stale-bound 'which the model does not have'

cp -R quint "$controls/no-apply"
{
    header OneActiveLeasePerPocket
    printf '%s\n' '--- a/quint/pachinko.qnt' '+++ b/quint/pachinko.qnt' '@@ -1 +1 @@' \
        '-this line is not in the model' '+nor is this one'
} >| "$controls/no-apply/mutants/qm03-control.patch"
refuses no-apply 'does not apply'

appended ill-typed 'module qm03_control { val broken: int = "not an int" }'
refuses ill-typed 'does not typecheck'

appended survivor '// a comment changes nothing a property can see'
refuses survivor 'survives'

cp -R quint "$controls/wrong-property"
test -f "$first"
grep '^# property: ' "$first"
if grep -Fx "# property: $other" "$first"; then exit 1; else test "$?" -eq 1; fi
sed "s/^# property: .*/# property: $other/" "$first" >| "$controls/wrong-property/mutants/$(basename "$first")"
refuses wrong-property 'survives'

sh ai_tmp/qm03/make-controls.sh "$controls"
refuses canary-true 'Canary was not violated'
refuses new-property 'violated on the unchanged model'
grep -F 'ScratchFalse' "$controls/new-property.log"
refuses late-property 'violated on the unchanged model'
grep -F 'ScratchLate' "$controls/late-property.log"
grep -F 'Canary violated' "$controls/late-property.log"
refuses instance-property 'in an instance'
grep -F 'ScratchInstance' "$controls/instance-property.failed"
refuses no-witness 'has no witness'
refuses stray-witness 'names no rule'
refuses unreachable 'was not reached'
refuses drifted 'does not apply'
refuses edits-property 'changes the property it names'
refuses deadlock 'not a counterexample'

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The first `just verify-quint` after the listener and every `refuses` line are the red
tests; `green.log` is the green one. A control that fails to build is not a pass: if a
`sed` matches nothing, the copy is still a good model, the recipe passes on it, and
`refuses` exits 1. `refuses` looks for its text on the recipe's own
`verify-quint: FAILED:` line, not anywhere in the log, so it is positive evidence of the
reason, not only of the failure, and a name that an earlier diagnostic happens to echo
does not satisfy it. `grep '^# property: '` is the positive control for the `sed` that
follows it, and for the line between them, which stops the script if the patch already
names `other`. In the late-property control the line `Canary violated` shows that the
canary run had passed, so it was the pass that reported `ScratchLate`. `test -z "$status"`
is the one probe that passes by printing nothing; `just check` before it ends with its own
line. The `|| true` after `wait` covers only the killed listener, and no claim rests on
it. If claim F found that the deadlock mutant survives, the last `refuses` reads
`survives`, and Departures says so.

Step 6 holds the five controls this script leaves out, the `just fix` and `just lint`
evidence for the patches, and step 7 the network denial and the timings. Step 13's CI
run is quoted beside this script's output, not inside it.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1

### Checkpoint 2

### Checkpoint 3

### Time budget

### Departures

### Verification

## Open points

- **Two branches without a control.** The check that a run's log shows Quint starting
  its own server, and the wait for the port after a run, are not exercised end to end:
  each needs a server to appear or linger at a moment no control can fix. Checkpoint 2
  reviews them by reading. A test hook in the script would exercise them and would also
  be a way to switch them off. Nobody is assigned.
- **A witness on a deadlock's trace.** The CLI would print `[observed]` for a deadlock's
  trace that holds the witness's state (`verify.js`, lines 269 to 317), and no probe
  produces one: a witness run stops at the witness's state first. If claim F finds
  that Apalache reports no deadlock under Quint's configuration, the pass does not see
  one either, the "deadlock" control reads `survives`, and the recipe's branch for
  `reached a deadlock` joins the branches without a control. The model's freedom from
  deadlock then rests on reading `step` (README, "Every step is total"). The
  maintainer decides at step 4's **Stop**, with claim F's output, whether that is
  enough or whether a later ticket sets Apalache's deadlock option.
- **How QM02 wrote the model.** This ticket was drafted before QM02 ran. Point 6's
  rule rests on the README's naming, but the edits of point 12 and the builds of step
  6's "script" controls depend on the model's text. The builder fits them at step 2,
  and the maintainer rules at step 4. A model the helper refuses is a **Stop**.
- **The CI figure needs a push.** Claim 12's CI half cannot be had without one, and the
  ticket cannot authorise it. The maintainer authorises it at step 12, or says that the
  ticket hands back with the CI figure open, which this ticket's acceptance criteria do
  not allow as written.
- **Two worktrees.** The recipe refuses a port that is busy before or after a run, and
  a run whose log shows no launch of its own; it does not prevent two recipes from
  racing for the port, and it does not show which process answered. The lock of point
  4 would close the race between worktrees. The maintainer decides at step 4.
- **Quint's connection and the spawned process.** Carried from AP01's Open points: the
  log shows Quint spawned a server and found none before, not that its connection
  reached that process. Point 4 gives the race that every probe and log line passes.
  The wait after a run is for the port, not for the pid, so a spawned process that
  outlives its listener is not seen either. Nobody is assigned.
- **A run that connects more than once.** Point 4's rule against the connecting line
  fits every run of this ticket. QM13's inductive run connects once for each phase and
  needs the rule point 4 states for it. Settled by QM13, in the recipe and in its own
  controls.
- **A server already running.** No record this ticket cites shows the pinned CLI run
  against an Apalache server that was already listening, and this ticket does not run
  it: shared rule 10 forbids a `verify` with the port in use. The connecting line is
  read in the source, and the recipe's refusal of a busy port is what keeps the case
  out. The maintainer decides whether one probe, against a server started by hand from
  this worktree's stamped install, is worth an exception.
- **A deleted mutant.** Point 2 says what shows a patch removed by a later change, and
  the alternative. The maintainer decides at step 4.
- **No watchdog.** Quint has no timeout, and the recipe adds none: POSIX `sh` has no
  portable one. The job's `timeout-minutes` bounds a hung run in CI; locally the trap
  cleans up after an interrupt and the next run's probe finds a leftover server. The
  maintainer decides whether that is enough when a later ticket's runs get long.
- **The denial on Linux.** Step 7's denial is run on the builder's machine. The CI run
  is not network-denied after its installs, so Linux has the recipe's own evidence and
  no OS-level denial unless the builder has a Linux machine. The maintainer decides
  whether that needs a step in the job.
- **Releases.** `.github/workflows/release.yml` runs `just check`, which does not
  verify. It checks that the tag's commit has reached `main` and never reads that
  commit's CI status. So a release is verified only as far as the required `check` job
  gated the merge that brought its commit to `main`, and a tag on a commit older than
  this ticket never ran the `quint` job. Whether a release should verify again is the
  maintainer's, outside this ticket.
- **`AGENTS.md`.** Whether the agent contract should say, beside QM02's same-change
  rule, that `just verify-quint` is never run in two worktrees at once. The maintainer
  decides at step 4; until then the row in Files touched adds the recipe's name only.
- **`_apalache-out/` grows.** Every `verify` leaves a run directory there. The recipe
  does not remove it, because other runs' output may be in it. Nobody is assigned.
