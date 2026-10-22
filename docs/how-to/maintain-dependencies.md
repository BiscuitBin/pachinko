---
title: "Maintain dependencies"
kind: "how-to"
audience: [maintainer, agent]
canonical_for: [dependency_maintenance]
requires: []
---

# Maintain dependencies

This guide will describe how dependencies are added, updated and audited: Cargo
dependencies under the caret-range policy with a committed lockfile, the Python tooling
through pixi, and the pinned command-line tools. Until the automated update ticket
lands, it will say plainly that nothing updates them on its own and a maintainer does
it by hand. Lane T07 writes it.
