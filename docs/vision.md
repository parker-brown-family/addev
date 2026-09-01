# Vision

Date: 2026-09-01
Status: seed — the shape, not a specification

Carried out of `docs/parked/addev-open-source-tier.md` in the Conclave
repository, where this idea was deliberately parked on 2026-08-31 and unparked on
2026-09-01 by `0005 — Two products`. That record holds the reasoning for both
decisions and the constraints this repository inherits.

---

## The one-sentence version

An agentic development environment for one person, on their own machine, built
into Omarchy.

## The problem it addresses

A single developer running coding agents now produces more work than they can
review. The bottleneck moved from writing code to deciding what deserves
attention next, and the tooling did not follow: what exists is macOS-first,
built for teams, and routes source through a vendor's cloud.

None of those three properties helps a person working alone on Linux, and the
third actively disqualifies the tools for anyone who will not send their code
off the machine.

## What it is

- **Open source, MIT.** Community-facing.
- **Local only.** No account, no server, no telemetry. Not as a privacy feature
  but because a second machine is not part of the problem.
- **Lightweight.** No collaboration surface, no comms port, no approval
  workflow. The features that exist to serve teams are exactly what a solo
  developer does not need, and carrying them is what makes team tools feel heavy
  to one person.
- **Omarchy-native.** One command to install, themed with the system, bound to
  its keys. Not a generic Linux tool that runs there.
- **In the Delight family.** Alongside Terminal Delight, Markdown Delight and
  Context Delight, sharing their posture rather than their code.

## Why Omarchy specifically

A generic agentic environment competes with everyone and is distinguished by
nothing. An environment built for one opinionated, actively-used desktop has a
community to build for, a coherent set of conventions to adopt rather than
invent, and no incumbent occupying the slot.

It is also the family's existing posture: read the operating system you are
actually on, and take its conventions.

## What is deliberately unresolved

**Everything below the shape.** No feature is specified here and none should be.
When work starts, it starts with a decision record.

Three questions that will need answering early, recorded so they are not
rediscovered:

- **What form does the Omarchy integration take** — a plugin, a package, an
  install script, a Hyprland-aware daemon? This is the thing that distinguishes
  the product, so it deserves a decision rather than an assumption.
- **What does a solo developer's attention surface look like?** The team answer
  is review gates and ranked elimination. That is too much ceremony for one
  person. The right answer is probably different in kind, not smaller in degree.
- **Does anyone want this?** No evidence has been gathered. `#7 — Customer
  shape` in the Conclave repository is open and was explicitly overridden rather
  than satisfied when this was unparked. Treat the market as unvalidated.

## Constraints inherited from `0005 — Two products`

- ADDev does not shape Conclave's architecture, and Conclave does not shape
  ADDev's.
- No abstraction is shared between them until a second implementation demands
  it.
- Nothing from Conclave or `counterpoint-research` enters this repository, which
  is MIT-licensed and will be public. See `AGENTS.md`.
