---
title: "Security model"
kind: "explanation"
audience: [user, contributor, maintainer, operator, agent]
canonical_for: [security_model]
requires: []
---

# Security model

This page will set out what a library and a minimal service binary defend and what they
leave to their callers: the supply chain and how cargo-deny guards it, the ban on unsafe
code in both crates, and the effects boundary that keeps the library from performing
effects itself. The network surface of the service is deferred, and the page will say
so until a decision records one. Lane T07 writes it.
