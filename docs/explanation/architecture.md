---
title: "Architecture"
kind: "explanation"
audience: [contributor, maintainer, operator, agent]
canonical_for: [system_architecture]
requires: []
---

# Architecture

This page will explain how the workspace is put together: the split between the library
and the service, the effects boundary that keeps the library from touching the outside
world and the service that implements it, what is deferred until the specifications
exist, and what the container image and release tickets add when they land. Each of
those tickets updates this page as it merges. Lane T07 writes it.
