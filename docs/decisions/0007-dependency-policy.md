---
title: "Decision 0007: Dependency policy: caret ranges and `Cargo.lock`"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_dependency_policy]
requires: []
---

# Decision 0007: Dependency policy: caret ranges and `Cargo.lock`

*Carried from libpawdoku's decision 0007 at `ccdd001`.*

This record will state how dependencies are declared and pinned: caret ranges in the
crate manifests, and a committed lockfile that the gate checks is current. It will
explain why both together suit a workspace holding a library and a binary, and cover the
context, the decision, its consequences and what would reopen it.

This page is a stub that the foundation ticket created so the record has its number,
title and topic from the first commit. Lane T07 writes the full record in the shape the
index describes: context, the decision, consequences, what would reopen it, and related
pages.
