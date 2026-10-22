---
title: "Layering and dependency direction"
kind: "explanation"
audience: [contributor, maintainer, agent]
canonical_for: [dependency_boundaries]
requires: []
---

# Layering and dependency direction

This page will explain which code may depend on which. Between crates the direction is
one way, the service on the library, and Cargo enforces it. Within the library, module
direction will follow the import graph of the specifications once there is one; until
then there is a single module and nothing to order. Lane T07 writes it.
