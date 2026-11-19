---
id: PD07
title: "Python double: is the inspection shape part of the contract?"
status: todo
depends_on: [PD06]
parallel_with: []
branch: ticket/pd07-inspection-contract
estimated_size: S
---

# PD07: Python double: is the inspection shape part of the contract?

## Context

Two Rust tests the series ported assert facts about the Rust's inspection snapshot that
`docs/specs/pachinko.allium` does not state, and a double that follows the specification
fails them when they are ported literally:

- **S053** (PD03, spec note N24). The Rust's test asserts that an ended lease leaves the
  registry: its snapshot's lease list is empty. The specification's `Lease` ends
  `released` or `expired`, both terminal, with `ended_at`, and no rule removes it. The
  double keeps the ended lease and ports the specification's facts instead.
- **S091** (PD04, spec note N31). The Rust's test lists a pocket's records in its own
  snapshot order, `[("b", 3), ("z", 1)]`. The specification treats a pocket's records as
  a set, and no rule asks them to be listed. The double keeps insertion order, so it
  lists `[("z", 1), ("b", 3)]`. The test compares positions by key.

The maintainer accepted both ports for now (PD03's and PD04's hand-backs record the
rulings) and asked for this follow-up, to investigate whether the inspection shape
belongs in the contract.

## Goal

A written answer, for the maintainer to accept or reject, to two questions:

1. Is an ended lease part of the observable state, or is the registry of leases an
   implementation detail? What does fencing by lease id need kept, and for how long?
2. Is the order in which an inspection lists a pocket's records part of the contract? If
   so, which order: by key, by position, or another?

## Non-goals

- No change under `docs/specs/`. A finding becomes a proposal for the maintainer, who
  moves the specification first if they accept it (`spec-change`).
- No change under `crates/`.
- No change to the double's behaviour until the maintainer rules.

## Steps

1. Read N24 and N31 in `double/SPEC-NOTES.md`, the two hand-backs, the specification's
   `Lease` entity and the inspection surface, and the catalog rows S053 and S091.
2. List every other ported test whose assertion depends on the inspection shape: an
   order, the presence of an ended entity, or a derived set. Use
   `rg -n 'impl-choice|N24|N31' double/tests`.
3. For each question, write the options with their consequences for the Rust, the double
   and the catalog. Recommend one.
4. Put the recommendation to the maintainer. Stop.

## Acceptance criteria

- Both questions have a recommendation, with the evidence from step 2.
- The maintainer's answer is recorded in the hand-back notes.

## Verification

```sh
set -eu
git rev-parse HEAD
just python-double
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

Written at the end.

## Open points

None yet.
</content>
</invoke>
