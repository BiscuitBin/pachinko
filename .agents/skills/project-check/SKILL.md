---
name: project-check
description: Bring a workspace to a state where the full gate runs, and interpret what it reports.
---

This skill is a stub that ticket T05 replaces. Until then, follow `AGENTS.md`, which is canonical and wins wherever the two disagree.

When T05 writes it, this file will hold the procedure for bringing a workspace to the point where the full gate runs, and for reading what each of the nineteen gates reports, in the order this repository's runner calls them.

The recipes it will name include `just initialize` and `just check`. Run the narrowest of them while working, then `just check` before handing the work back.
