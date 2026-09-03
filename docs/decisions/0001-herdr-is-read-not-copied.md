# 0001 — herdr is read, not copied

Date: 2026-09-03
Status: accepted

## Context

herdr (`herdrdev/herdr`) is a terminal workspace manager whose declared user is a
coding agent rather than a human. It holds the PTYs in a background server,
classifies what is running in each pane as working / blocked / done / idle, and
rolls that state up so an operator running many agents can see which one is
stuck. It is the closest existing thing to what this repository proposes, and it
was read carefully on 2026-09-02 before any decision here was made.

It is Apache-2.0. This repository is MIT and will be public.

That asymmetry is one-directional and it is not a formality. Apache-2.0 source
cannot be moved into an MIT repository without its terms travelling with it —
the licence text, the attribution, the patent grant. A public git history cannot
be un-published afterwards, which means the mistake is not correctable by
deleting the file.

The same discipline already applies to Conclave, for a different reason:
disclosure rather than licence. `AGENTS.md` states it. This record states the
licence half, because the two failures look identical from inside a session and
the reasoning that prevents them is not the same.

## Decision

ADDev reads herdr and copies nothing from it.

Facts and interfaces are not copyrightable and are fair to re-derive. What a
program does, the shape of a protocol, the observation that per-agent rules
belong in data rather than in compiled logic — none of that is anyone's
property, and this repository is free to arrive at the same conclusions.

Expression is. No herdr file, function, rule set, manifest, schema, comment or
paragraph is transcribed, adapted, translated into another language, or used as
a working reference while the corresponding thing is written here. Anything in
ADDev informed by herdr is written from a description of the behaviour, never
from the source in front of the author.

The operational form of that, for both the human and any agent working in this
repository: a session that has had herdr source open does not then write the
ADDev code implementing the same capability. Same clean room as any other, and
for the same reason — the risk is not deliberate copying, it is reproducing the
shape of something that happened to be open in the next pane.

## Interoperation is not derivation

Where herdr's interface is the thing being interoperated with — the keys in a
plugin manifest it will parse, the names of events it emits, the path of a socket
it creates — ADDev may match it exactly and by name. An interface you must match
in order to talk to a program is a fact about that program, not an expression of
it. `omarchy-herd` already does this and is unaffected by this record.

The boundary is the direction of the copying. Reading herdr's published event
names so that something can subscribe to them is interoperation. Reading herdr's
detection manifests so that ADDev ships equivalent ones is derivation, and is
refused.

## No pull requests to herdr

Recorded here so it is not rediscovered by someone preparing one. herdr's
`CONTRIBUTING.md` states that the project opened its pull request gate and closed
it again, because agent-authored drive-by patches moved the whole verification
cost onto the maintainers. Implementation pull requests are accepted only from
accounts already listed in `.github/APPROVED_CONTRIBUTORS` or
`.github/MAINTAINERS`; everything else is closed automatically, regardless of
quality, and the file says explicitly not to ask to be added.

The sanctioned paths are a reproduced bug report, a Discussion, and an
out-of-tree plugin. All three remain open to this project and none of them are
affected by the rule above.

## Consequences

- Re-derivation costs time that copying would not. That cost is accepted.
- Naming diverges deliberately. Where a herdr term would be the obvious one,
  ADDev picks its own, because matching vocabulary invites matching structure.
- A future record that adopts a herdr-shaped idea cites this one and says which
  half it is relying on: the fact, or the interface.
