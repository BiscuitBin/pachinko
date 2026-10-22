---
title: "Decision 0001: A library and a service in one workspace"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_library_and_service]
requires: []
---

# Decision 0001: A library and a service in one workspace

This record will explain why the repository is one workspace of two crates: a std
library that holds the domain once the specifications define one, and a minimal service
binary that depends on it and supplies the real implementations of its effect traits. It
will cover the context, the decision, the consequences and what would reopen it.

This page is a stub that the foundation ticket created so the record has its number,
title and topic from the first commit. Lane T07 writes the full record in the shape the
index describes: context, the decision, consequences, what would reopen it, and related
pages.
