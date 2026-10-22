---
name: fix-quality
description: Diagnose and repair a failing quality gate at its root instead of suppressing the finding.
---

This skill is a stub that ticket T05 replaces. Until then, follow `AGENTS.md`, which is canonical and wins wherever the two disagree.

When T05 writes it, this file will hold the procedure for diagnosing and repairing a failing quality gate at its root rather than suppressing the finding, with one dispatch line for every gate this repository runs, not only the ones the upstream library had.

The recipes it will name include `just check` and `just fix`. Run the narrowest of them while working, then `just check` before handing the work back.
