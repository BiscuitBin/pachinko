# worker-pool: the execution-model spike

This crate is the prototype ticket T20 (`tickets/T20-worker-pool-spike.md`) built to
measure the maintainer's proposed execution model for the domain
`docs/specs/pachinko.allium` specifies: a bounded number of long-lived worker threads,
each running single-threaded state machines over partitions of pockets, with the pocket
order and the lease routing in one order machine and the pockets' records and leases in
the pocket machines.

## What it is

- Every rule of the module as a branch of a step function in a machine that takes `now`
  as a value and never reads a clock, spawns, sleeps or locks.
- A pool of worker threads under `std::thread::scope`, bounded queues between them, a
  clock that stamps each input, and a tick injected into every machine.
- Tests that prove replay, the module's invariants, one thread per partition, the thread
  bound, structured stop with panic propagation, the fencing token and freedom from the
  two-full-queues deadlock.
- A demo binary that measures one machine on one thread against partitions over several
  worker threads and prints one table.

## What it is not

It is not the implementation, not a conformance claim and not a decision. Its findings
are the ticket's hand-back notes; the record that supersedes decision 0014 is the
maintainer's to take from them, and the implementation ticket starts from the module and
`just plan-spec pachinko`, never from this code. Nothing under `crates/` or `docs/specs/`
depends on it.

## Running it

- `just spike` runs the demo binary in release mode, with no arguments the default sweep,
  otherwise the arguments through; `just spike --help` prints them.
- `just spike-test` runs its unit, integration and doc tests.

Both sit outside `just check`, which lints the crate (lock, format, clippy and deny) but
does not run it. The crate is its own Cargo root with its own `Cargo.lock`, so the root
workspace is untouched.

The table's preamble defines its thresholds, its `busy %` (step time only) and the
invariants checked at the end. Each row is one whole run: the median of the repeats by
`pub/s`, or the first repeat that failed an invariant, timed out or lost a reply; the
row's `unhealthy repeats K of R` counts every such repeat, so no repeat's failure is
dropped, and the binary exits 1 when any row has one; a client thread's panic fails the
run. Rates divide by the window the clients measured over. Run-to-run noise is a few
per cent, so a ranking within that is not stable between runs; `--publishers` and
`--consumers` give a client-scaling run that shows whether the client count moves the
numbers.
