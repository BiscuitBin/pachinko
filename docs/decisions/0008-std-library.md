---
title: "Decision 0008: A std library, with effects kept out mechanically"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_std_library]
requires: []
---

# Decision 0008: A std library, with effects kept out mechanically

This record will explain why the library uses the standard library rather than going
without it, and what keeps effects out of it instead: a gate that checks the dependency
tree and the source of the library against a list of bans, run by `just effects`. It
will cover the context, the decision, its consequences and what would reopen it.

This page is a stub that the foundation ticket created so the record has its number,
title and topic from the first commit. Lane T07 writes the full record in the shape the
index describes: context, the decision, consequences, what would reopen it, and related
pages.
