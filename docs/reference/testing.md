---
title: "Testing"
kind: "reference"
audience: [contributor, maintainer, agent]
canonical_for: [testing_reference]
requires: []
---

# Testing

This page will state the testing rules: nextest as the runner, doctests, the test that
runs the built service binary, what coverage measures and its floor, fakes reached
through the effect traits rather than real effects, and the snapshot rule, which waits
for the first value worth snapshotting before the snapshot crate is added. Lane T07
writes it.
