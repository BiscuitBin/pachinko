---
id: T44
title: "Research the width of a rank against primary sources, and put the options to the maintainer"
status: todo
depends_on: []
parallel_with: []
branch: ticket/t44-rank-width-research
estimated_size: M
---

# T44: Research the width of a rank against primary sources, and put the options to the maintainer

## Context

A rank is the number a publisher supplies to order the versions of one key; higher wins.
`docs/specs/pachinko.allium` declares it an `Integer` and states no bound. The Rust
library holds it in 64 signed bits, the service writes it as a bare JSON number, and the
Python double holds it in an unbounded `int`. So the three disagree on which ranks
exist: the double accepts a rank the Rust cannot hold.

The review of the Python double raised this as part of its finding 13, "Should the
specification fix its numeric domains?", in `double/SPEC-NOTES.md`, with the notes N03
and N17 behind it:

- **N03, a rank is an unbounded integer.** The specification says a rank is an
  `Integer`, with no bound stated. The double's rank is a Python `int`, so negative, zero
  and very large ranks all order as integers. The note's question: should the
  specification bound a rank? An implementation with a fixed-width integer refuses or
  wraps a rank the double accepts.
- **N17, S019's large ranks are a 64-bit integer's extremes, not the double's.** The
  double ports scenario S019 with the Rust test's ranks, `-(2**63)`, `-(2**63) + 1`,
  `2**63 - 2` and `2**63 - 1`. Those are not a boundary for the double: it would accept
  `2**63` and order it above them, where the Rust's rank cannot hold it. The note's
  question is N03's: should the specification bound a rank, so that S019's values become
  the boundary for every implementation?

This ticket is research. It changes no behaviour, no specification and no code. It reads
primary sources, writes what they say into the handbook's prior-art page, puts four
options to the maintainer, records the maintainer's ruling, and names what a follow-up
would change. It ends there.

Nothing in this ticket re-opens the ruling below. A finding that the ruling is wrong
goes to the maintainer as a question, never into a recommendation that assumes it.

### The ruling on finding 13's rank question (2026-11-24)

**Bound a rank, but research the width first.** That a rank is to be bounded is decided.
Which bound is not. The maintainer gave no reason beyond wanting the width researched
before it is chosen; this ticket does not supply one.

- The research verifies, against primary sources, what similar systems use for a
  caller-supplied "higher wins" number, and why. The sources the ruling names:
  Elasticsearch external versioning, Cassandra write timestamps, etcd, Kafka, NATS,
  MongoDB cluster time, PostgreSQL's LSN and `xid8`, Protobuf's JSON mapping, and RFC
  8259 section 6.
- It adds a section to `docs/explanation/prior-art.md`.
- It puts these options to the maintainer, and no others unless the research finds one:
  - **A.** 64-bit signed, as built.
  - **B.** 53-bit safe integers.
  - **C.** 64-bit signed, with the rank as a string on the wire.
  - **D.** Unsigned 64-bit.
- **No bound is stated in the specification until the maintainer rules** on those
  options. This ticket therefore edits nothing under `docs/specs/`.

Finding 13 asked two other things, and both were ruled elsewhere on the same day: the
queue's first position goes to `tickets/T47-start-rule-and-config-bounds.md`, and the
time question to `tickets/T42-clock-model.md`. Neither is this ticket's.

### Nothing here about another system is evidence yet

When the ruling was made, a comparison table of the systems above existed only as
recollection. None of it was checked, and none of it is in this ticket. The rules for
the research:

- **Every claim about another system cites its primary source**: the system's official
  documentation, its specification, or its source code, with the URL fetched and the
  date it was read. A blog post, a forum answer, a secondary summary or an assistant's
  recollection is not a source.
- **A claim that cannot be sourced is reported as unverified, never asserted.** It goes
  into the hand-back and, if it matters to an option, into the page as "not found" or
  "not verified", in the page's existing sense that the pages read are silent.
- **This ticket gives no URLs.** The executor finds each authority's own page and
  records the URL it actually fetched. A URL typed from memory is the same defect as a
  fact typed from memory.
- **Fetching needs the network.** The reads are an authorised step of this ticket
  (step 3), read-only, and they send nothing about this repository.

One example was offered with the ruling, from memory, and is **unverified**: that
MongoDB's timestamp type packs seconds into the top bits of a 64-bit value, so that a
signed 64-bit rank could not hold such a value from the year 2038. Step 3 verifies it
against MongoDB's own documentation or specification, or drops it. It appears in the
page only if verified, with its source.

Two statements below are arithmetic, not claims about a system, and need no source:

- A binary64 floating-point number has a 53-bit significand, so it represents every
  integer of magnitude up to 2^53 and not every integer above it: 2^53 + 1 has no
  binary64 value. A JSON client that reads numbers into binary64 therefore cannot
  represent every integer above 2^53. Which clients do so, and what the JSON
  specification says about it, are research questions (RFC 8259 section 6).
- A signed 64-bit integer holds -2^63 to 2^63 - 1. It cannot hold an unsigned 64-bit
  value whose top bit is set, such as one that packs a field into the top bits, nor
  anything wider than 64 bits.

### What exists today

Re-read each at step 2; these were true when the ticket was written, on the branch
`build-python-double`. They are cited by item, not by line.

- **The specification.** `Record.rank` and `Record.held_rank` (the latter present while
  `status = held`), `SupersededRecord.rank` and `DeadLetter.rank` are each `Integer`.
  The `Publish` trigger carries `rank`, and the Publish rules compare it with
  `rank > record.rank` and `rank < record.rank`. The module's glossary defines Rank as
  "the publisher-supplied order between the versions of one key; higher wins". No
  invariant or comment bounds a rank.
- **The library.** `Rank` in `crates/libpachinko/src/ids.rs` is `pub struct Rank(i64)`,
  built by `Rank::new(i64)` and read by `Rank::get() -> i64`. Its doc comment says it
  "orders like the `i64` it holds, negatives and zero included", and its doctests use
  negative ranks and `i64::MAX`.
- **The service reads a rank as a bare JSON number.** `PublishBody` in
  `crates/pachinko/src/http.rs` and `BatchRow` in `crates/pachinko/src/http/batch.rs`
  each declare `rank: i64` and derive `Deserialize`.
- **The service writes a rank as a bare JSON number.** `RecordReply` in
  `crates/pachinko/src/http.rs` (a pull's batch) and `LeaseRecordReply` in
  `crates/pachinko/src/http/lease_read.rs` (the lease read) each declare `rank: i64`,
  derive `Serialize` and fill it from `record.rank.get()`.
- **The handbook says so.** `docs/reference/http-api.md`, under its routes table:
  "`rank` is a JSON integer read as a signed 64-bit number, so every value from
  `-9223372036854775808` to `9223372036854775807` round-trips exactly." That sentence is
  true of the service and of a client that keeps 64-bit integers. It says nothing about
  a client that reads JSON numbers into binary64.
- **Scenario S019**, "Large ranks keep full precision across supersede and stale", tagged
  `wire` in `docs/reference/testing.md`. The library test
  `large_ranks_keep_full_precision_across_supersede_and_stale` in
  `crates/libpachinko/tests/queue/publish.rs` publishes `i64::MIN`, `i64::MIN + 1`,
  `i64::MAX - 1` and `i64::MAX`. The wire driver `large_ranks` in
  `crates/pachinko/tests/http.rs` publishes `i64::MIN`, `i64::MAX - 1` and `i64::MAX` and asserts that
  the batch's text contains `9223372036854775807`. Every one of these values is beyond
  53 bits, and half are negative.
- **The double.** A rank is an unbounded Python `int`
  (`double/pachinko_double/model.py`: "ranks and positions are unbounded `int` (N03)").
  `test_large_ranks_keep_full_precision_across_supersede_and_stale` in
  `double/tests/test_publish.py` ports S019 with `-(2**63)` and `2**63 - 1`, under a
  comment that they are the Rust test's extremes and not the double's.
- **The prior-art page.** `docs/explanation/prior-art.md` is registered in
  `docs/manifest.yml` with `canonical_for: ["prior_art"]`. It records a survey "made on
  2026-11-12", grades each claim by the reading behind it ("How to read the evidence"),
  already cites Elasticsearch external versioning for the rank rule and for an equal
  rank, holds a section "Evidence for the open questions", and ends with "What this
  page does not establish". It says nothing about the width of a rank or of any other
  system's version number. `docs/reference/documentation-contract.md` ("Links that
  leave the repository") names it as one of four pages that link outward, each link to
  a whole page and never to a heading.
- **Where `double/` lives.** The double and `double/SPEC-NOTES.md` exist on the branch
  `build-python-double`. They were not on `main` when this ticket was written. This
  ticket reads them and edits neither, and everything it needs from N03 and N17 is
  restated above.

**Not verified when this ticket was written:** what the service answers to a publish
whose `rank` is a JSON number outside the signed 64-bit range, or a JSON number with a
fraction or an exponent. `docs/reference/http-api.md` lists 400 `bad_request` for a body
that "is not the JSON the route takes"; whether such a rank lands there was not read or
run. Step 2 establishes it.

### Risks (`CONVENTIONS.md` §13)

- **A claim from memory in a handbook page.** The page's value is that each statement
  is a product's own documentation as read on a stated day. One unsourced row spoils
  it. Every row of the new section has its URL and date, or says "not verified".
- **A source that does not say why.** The ruling asks what the systems use *and why*. A
  width with no stated reason is recorded as a width with "reason not stated in the
  source"; the executor does not supply the reason.
- **Number caller-supplied or system-assigned.** Several of the named systems may
  assign their number themselves. Whether each one's number is supplied by the caller is
  itself a research question, and a system-assigned number is weaker precedent for a
  rank; the page says which each is.
- **A recommendation that reads as a ruling.** The section and the hand-back lay out
  the options. If the executor gives a recommendation it is labelled as one, and no
  text outside this ticket says a bound has been chosen until the maintainer rules.
- **Statements the new section makes false.** The page's introduction dates the whole
  survey to 2026-11-12, and its evidence grades and "What this page does not establish"
  describe that survey. A section read on a later day, by a different method, must not
  be covered by those sentences as they stand.
- **The network at first run, none in `check`.** Step 3's reads need the network and are
  authorised there. `just check` resolves no `https://` link; `just check-links-online`
  does, and this ticket runs it only at step 5, where it is authorised.
- **lychee and typos over `tickets/`.** The hand-back writes paths and URLs as code
  spans, never as links. In the handbook page they are Markdown links, as the page's
  existing citations are.

## Goal

`docs/explanation/prior-art.md` holds a section on the width of a caller-supplied
ordering number: for each source the ruling names, what the source itself says, with
its URL and the date read, or "not verified"; and for each of the options A to D, what
it would mean for this queue, from the repository's own facts. The maintainer has been
given the options with that evidence, and the ruling is quoted and dated in this
ticket's hand-back, with the list of what a follow-up ticket would change. The
specification, the crates and the double are byte-for-byte what they were.

## Non-goals

- No bound in `docs/specs/pachinko.allium`, and no other edit there.
- No change under `crates/` or `double/`, to `docs/reference/http-api.md`, to
  `docs/reference/testing.md` or to `double/SPEC-NOTES.md`. S019 keeps its values.
- No decision record. The ruling this ticket ends with is recorded in the hand-back;
  the follow-up that lands it writes any record it needs.
- No follow-up ticket file. This ticket names what the follow-up would change; creating
  that ticket and its row in `tickets/README.md` is asked of the maintainer at step 6,
  not assumed.
- No bound on a position. N03 notes that positions are unbounded in the double too; the
  ruling covers a rank only.
- No answer to what a publish with a rank outside the bound is refused with. That
  belongs with the named refusals of `tickets/T43-caller-contract.md`; see Open points.
- Nothing installed and nothing run from another system. The research reads
  documentation, specifications and, where they are the primary source, source files.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/explanation/prior-art.md` | shared, own lines | The new section; the introduction's survey date, "How to read the evidence" and "What this page does not establish", each reconciled with a section read on a later day; a line under "Related pages" only if the section cites a page not yet listed |
| `tickets/README.md` | shared, own lines | T44's Status cell; the follow-up ticket's row, only if the maintainer asks for that ticket at step 6 |
| One new file under `tickets/` | new, only if the maintainer asks for it at step 6 | The follow-up ticket that lands the ruled bound |
| `tickets/T44-rank-width-research.md` | ticket | `status:` and hand-back notes |

`docs/manifest.yml` and `docs/README.md` do not change: the page keeps its path, title,
kind, audience and its one topic, `prior_art`, which already covers this section.

## Steps

1. **The worktree and the baseline.** Create the branch and its worktree from `main`
   (`CONVENTIONS.md` §11) and run `just initialize` there (network, authorised by this
   step). Run `just check` on the untouched worktree and quote its last line as the
   baseline. If the base holds no `docs/explanation/prior-art.md`, or lacks any item
   "What exists today" names other than `double/`, stop and ask the maintainer which
   base to use.
2. **The re-read.** Use the `plan-change` skill. Re-read every item under "What exists
   today" and record each as confirmed or corrected. Then establish the one fact the
   ticket could not: what the service answers to a publish whose `rank` is `2^63`, is
   `-2^63 - 1`, is `1.5`, and is `1e3`. Read the deserialiser's behaviour in the pinned
   source, and confirm it with a run against `target/debug/pachinko` after `just build`,
   using a scratch client under `ai_tmp/`; quote each request and reply. Nothing from
   this step is committed.
3. **The sources. Network, authorised by this step**, read-only. For each row below,
   find the authority's own page and answer the questions from that page alone. Record
   the URL fetched, the date, and the sentence or table row the answer rests on, quoted.
   Where a page arrives cut short or summarised by the tool that fetched it, say so, as
   the page's existing notes do. Raw fetches and notes stay in `ai_tmp/`.

   | Source the ruling names | Questions |
   | --- | --- |
   | Elasticsearch external versioning | The type and range of an external version; whether the caller supplies it; how it travels in a request; any stated reason for the range |
   | Cassandra write timestamps | The type, unit and range of a write's timestamp; whether a client may supply it; any stated reason |
   | etcd | Which numbers order writes (revision, version or other); their type; whether any is caller-supplied; how the JSON gateway, if one is documented, carries 64-bit integers |
   | Kafka | The type of an offset and of a record timestamp; whether either is caller-supplied; whether any documented feature keeps the highest of a caller-supplied number, and its type |
   | NATS | The type of a stream sequence; whether a caller supplies any ordering number, if the source documents one; how it travels |
   | MongoDB cluster time | The layout and width of the timestamp type and of cluster time; whether the unverified 2038 example above holds; how its JSON form carries it, if the source documents one |
   | PostgreSQL LSN | The type and width of an LSN, and its text form |
   | PostgreSQL `xid8` | Its width and signedness, and the reason the documentation states for it, if it states one |
   | Protobuf's JSON mapping | How the mapping carries 64-bit integers, signed and unsigned, and the stated reason |
   | RFC 8259 section 6 | What the section says of the range and precision of numbers, and of interoperability |

   A suggestion, not part of the ruling, and unverified: option B's "safe integer" is
   believed to be a term the ECMAScript specification defines. If the executor finds
   the definition there, it may cite it the same way, as the primary source for the
   bound 53 bits gives; if not, the page does not use the term as that specification's.

   Any source that cannot be reached, or does not answer a question, is recorded as
   "not verified" with what was tried. Nothing is filled from memory.
4. **The section.** Use the `review-docs` skill. Add one section to
   `docs/explanation/prior-art.md`, after "Evidence for the open questions" and before
   "What this page does not establish"; a suggested heading is "Evidence for the width
   of a rank". It holds:
   - why the question exists, in two or three sentences, linking the specification;
   - one table row or item for each source in step 3: what the number is, its type,
     width and signedness, whether the caller supplies it, its form on the wire or in
     JSON, the reason the source gives or "reason not stated in the source", the link,
     and the date read; each link to a whole page, never to a heading
     (`docs/reference/documentation-contract.md`, "Links that leave the repository");
   - the options A to D, each with what it would mean here, drawn only from this
     repository's facts and step 3's findings. At the least:

     | Option | From this repository's facts |
     | --- | --- |
     | A, 64-bit signed as built | No code or wire change; the specification gains a bound equal to today's `Rank`; S019's values become every implementation's boundary; a binary64 client still cannot represent every rank above 2^53 |
     | B, 53-bit safe integers | Every rank survives a binary64 client; ranks the library accepts today become out of range, S019's four values among them; whether negatives stay is part of the option |
     | C, 64-bit signed, a string on the wire | The library is unchanged; every request and reply shape that carries `rank` changes, a breaking change to the HTTP API; S019's wire driver changes |
     | D, unsigned 64-bit | Top-bit-set 64-bit values fit; negative ranks, which `Rank`'s doctests and S019 use today, do not; the wire question of A remains |

   - what was not verified, in the section or under "What this page does not
     establish".

   Then reconcile the sentences the section would make false: the introduction's "The
   survey was made on 2026-11-12" and "as read on that day"; "How to read the evidence",
   if the section's readings need a grade the list lacks; and "What this page does not
   establish". The section states its own date and method. It does not regrade or
   reword the earlier survey's claims, including the page's existing Elasticsearch
   citations; a conflict between an existing claim and what step 3 read is reported in
   the hand-back and put to the maintainer at step 6.

   Run `just check-docs`.

   **Adversarial review** (`CONVENTIONS.md` §14), over the section's diff and the
   hand-back's source list. The breakage reviewer re-fetches each cited URL and reports
   any row whose claim the page does not support, any reason the source does not state,
   any link to a heading, and any claim with no source. **Network, authorised by this
   step** for that re-read alone. A row the reviewer re-read and upheld may carry the
   page's "second reader" grade. If no reviewer could reach the network, say so in the
   hand-back, and every row stays "one reader". The conformance reviewer checks the section
   against the ruling's list of sources and options, and that no text says a bound has
   been chosen.
5. **Authorisation required: the outward links.** The section adds `https://` links
   that `just check` does not resolve. Ask whether to run `just check-links-online`
   (network). If authorised, run it and quote its result; if not, say in the hand-back
   that it was not run.
6. **Authorisation required: the maintainer's ruling on the width.** Commit the section
   on the ticket branch, then give the maintainer:
   - the four options, each with its consequences from the section, and any fifth
     option the research found, labelled as found and not as ruled;
   - the evidence table, and the list of what could not be verified;
   - step 2's answer on what the service does with an out-of-range or non-integer rank
     today;
   - these questions, none of which this ticket assumes an answer to: which option; for
     option B or D, whether negative ranks stay valid; whether the rank's form on the
     wire may differ from the specification's bound, as option C's does; whether a
     follow-up
     ticket should be written now, and whether it waits on
     `tickets/T43-caller-contract.md` for the name of the refusal of an out-of-range
     rank.

   Then stop until the maintainer answers. Quote the answer in the hand-back, dated. An
   answer that names something outside these points is a new design: record it and stop
   again. This ticket lands nothing the answer decides.
7. **What a follow-up would change.** Under the ruling, write in the hand-back the list
   a follow-up ticket would work from, re-read against the tree on the day. When this
   ticket was written the list was:
   - the specification, first (`AGENTS.md`, invariant 1): `Record.rank`,
     `Record.held_rank`, `SupersededRecord.rank`, `DeadLetter.rank` and the `Publish`
     trigger's `rank`, with the bound stated once and the others following from it; how
     Allium states a bounded integer was not checked when this ticket was written;
   - the library's `Rank` in `crates/libpachinko/src/ids.rs`, its constructor, doc
     comment and doctests;
   - the HTTP API: `PublishBody`, `BatchRow`, `RecordReply` and `LeaseRecordReply`, the
     answer to an out-of-range rank, and `docs/reference/http-api.md`'s sentence on
     `rank`;
   - the double: the rank fields of `double/pachinko_double/model.py`, the refusal or
     validation the bound needs, and N03's and N17's entries in `double/SPEC-NOTES.md`;
   - S019: its values in `crates/libpachinko/tests/queue/publish.rs`,
     `crates/pachinko/tests/http.rs` and `double/tests/test_publish.py`, its row in
     `docs/reference/testing.md`, and a scenario for a rank just outside the bound.

   If the maintainer asked at step 6 for the follow-up ticket to be written, that
   authorises one new file under `tickets/` and its row in `tickets/README.md`; add
   both to the hand-back's list of changed files. Otherwise write neither.
8. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
   hand-back notes.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Then use
   the `project-check` skill and run `just check`.

   **Authorisation required:** pushing the branch and opening the pull request are each
   asked for separately.

## Acceptance criteria

- The baseline line of `just check` is quoted, headed by its commit.
- Each item under "What exists today" is recorded as confirmed or corrected, and the
  service's answer to an out-of-range and to a non-integer rank is quoted from a run.
- The new section names every source the ruling lists. Each claim about a source has a
  link to that source's own page and the date it was read, or reads "not verified".
  No claim rests on a secondary source or on recollection.
- The MongoDB 2038 example is in the page with its source, or is absent from the page
  and reported in the hand-back as dropped, with what was read.
- Every outward link is to a whole page, never to a heading.
- The section sets out options A to D with their consequences for this queue and does
  not say one is chosen. A recommendation, if given, is labelled as the executor's.
- The page's introduction, its evidence grades and "What this page does not establish"
  are true of the page as it now stands.
- `git diff main...HEAD` touches no path under `docs/specs/`, `crates/` or `double/`.
- The maintainer's ruling at step 6 is quoted in the hand-back with its date, and
  nothing before it chose an option. The list of what a follow-up would change follows
  it.
- `just check-links-online` was run with authorisation and its result quoted, or the
  hand-back says it was not run.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). The
heading in the commands is the suggested one; use the heading step 4 chose.

```sh
set -o pipefail
just check-docs; echo "exit $?"
# positive control: the page is read and the new section's heading is found (exit 0)
rg -c '^## Evidence for the width of a rank' docs/explanation/prior-art.md; echo "exit $?"
# each source the ruling names appears inside the new section (each exit 0)
for source in Elasticsearch Cassandra etcd Kafka NATS MongoDB LSN xid8 Protobuf 'RFC 8259'; do
  sed -n '/^## Evidence for the width of a rank/,/^## What this page does not establish/p' \
    docs/explanation/prior-art.md | rg -c -F "${source}"; echo "${source}: exit $?"
done
# positive control: the diff against main is not empty and names the page (exit 0)
git diff --name-only main...HEAD | rg -c -F 'docs/explanation/prior-art.md'; echo "exit $?"
# the specification, the crates and the double are untouched. The diff is read into a
# file first, so a failed git read cannot pass as "no match": both must print "exit 0"
mkdir -p ai_tmp && git diff --name-only main...HEAD >| ai_tmp/t44-names.txt; echo "git diff: exit $?"
rg -n '^(docs/specs|crates|double)/' ai_tmp/t44-names.txt; [ "$?" -eq 1 ]; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Open points

- **The width itself** is step 6's to settle, with whether negative ranks stay valid
  and whether the rank's form on the wire may differ from the specification's bound.
  That the specification bounds a rank is ruled and is not asked.
- **What an out-of-range rank is refused with.** The maintainer has ruled that the
  specification names its refusals and fixes their order, and
  `tickets/T43-caller-contract.md` works the names out. A refusal for a rank outside
  the bound would be one more. This ticket reports what the service does today (step
  2) and asks at step 6 whether the follow-up waits on T43; it names no refusal.
- **How Allium states a bounded integer** was not checked when this ticket was written.
  It is the follow-up's to establish, under the `spec-change` skill.
- **Whether the follow-up ticket is written here** is asked at step 6.
- **The base branch.** The facts in Context were read on `build-python-double`. If
  `main` does not yet hold them when this ticket starts, step 1 stops and asks.

## Hand-back notes

### Baseline (step 1)

### The re-read, and the service's answer to an out-of-range rank (step 2)

### The sources: URL, date read and quoted text for each (step 3)

### What could not be verified (step 3)

### T44-1: the section (step 4)

### The outward links (step 5)

### The ruling (step 6)

### What a follow-up would change (step 7)

### T44-2: the whole diff, and the gate (step 8)
