# Herd — working contract

Canonical project behaviour file. `CLAUDE.md` is a symlink to this; add a symlink
for any new agent rather than copying content out.

Machine-global rules — narrative commits, no AI attribution, git identity by
remote, numbered artifacts named not numbered — live in
`~/.config/agents/AGENTS.md` and are not restated here.

## What this repository is

Herd, the local agent-state bus: one writer, many displays, and it draws
nothing itself. Open source, MIT, local only, no daemon. The `herd` binary lives
in `src/`; the displays that read its state file are separate repositories
(Crook on the Omarchy bar, Wool on the wall).

The repository began as **addev**, an umbrella name for an agentic development
environment, with the bus as a `crook/` subdirectory. The umbrella described
nothing anyone shipped and was dropped; the bus was promoted to the root and
took the name Herd. `docs/addev-seed-readme.md` keeps the original README as
history, and `docs/vision.md` still holds the wider shape for whatever earns
building next.

## The rule that matters most

**Nothing from Conclave enters this repository. Ever.**

Conclave is a proprietary product built by the same people for large teams. This
repository is MIT-licensed and will be public. A file, a function, a design
document or a paragraph moved from there to here is a licensing and disclosure
failure that cannot be undone by deleting it afterwards — public git history is
public.

The boundary is one-way and total:

- No Conclave source, at any length.
- No Conclave decision records, plans, measurements, customer material or
  research digests.
- No `counterpoint-research` digests or source records. That repository is
  private and its contents are proprietary research.
- Ideas arrived at independently are fine. Facts and interfaces are not
  copyrightable. But an idea *read* in Conclave and written down here is the
  failure this rule describes, not an exception to it.

If a session has had Conclave material in its context, it does not write here.
Same discipline as a clean room, and for the same reason: the risk is not
deliberate copying, it is reproducing the shape of something that happened to be
open.

## And the reverse

Herd does not shape Conclave's architecture. `0005 — Two products` in the
Conclave repository records this as the load-bearing constraint of having two
products at all. Nothing here is a requirement on anything there.

No abstraction is extracted from one to serve the other until a second
implementation genuinely demands it.

## Design posture

- **Read the operating system.** Omarchy and Hyprland have conventions. Take
  them — XDG paths, systemd user units, Unix sockets, file modes, the theme
  system — rather than inventing parallel ones.
- **One person is the user.** Any feature that only makes sense with a second
  person in the room belongs in the other product. No accounts, no server, no
  collaboration surface, no telemetry.
- **Low ceremony.** A solo developer's tolerance for process is the design
  constraint. If a workflow needs a review gate to be safe, it is the wrong
  workflow for this program.

## Status

Shipping. `herd` 0.1.1 is published as the `herd-bus` crate. Decision records go
in `docs/decisions/`, numbered and append-only like the sibling repository's,
and a change that carries a decision gets one before it gets code.
