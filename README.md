# Herd

**The local agent-state bus.** One writer, many displays; it draws nothing.

You run coding agents. Some are working, one finished ten minutes ago and is
waiting for you to look, one is sitting on a permission prompt, and one hung.
Every tool that shows you this today detects it separately — usually by
matching phrases on a screen — so the displays can't agree, and the first time
an agent vendor rewords a prompt, your status surface goes quiet *and reports
calm*.

Herd is the layer underneath: agents report their own state into one file, and
everything on your screen reads that file. The herd is the flock's state
itself; what you do with it is the rest of the family:

| | | |
|---|---|---|
| **Herd** | this — the bus | who exists, what state, how fresh |
| **[Crook](https://github.com/parker-brown-family/omarchy-crook)** | attention | the shepherd's hook: single out the one that needs you, from the bar |
| **[Wool](https://github.com/parker-brown-family/omarchy-agent-wool)** | presence | the whole flock's coat on one wall |

```
feed:  Claude Code hooks ─┐
       herd report ───────┼──→  ~/.local/state/herd/state.json  ──→  read: Crook, Wool, a tmux
       herd sync-herdr ───┘        (atomic renames, safe to watch)         line, herd watch scripts
```

## Install

```bash
cargo install herd-bus
```

(The package is `herd-bus` only because crates.io's `herd` is a name squatted
in 2022; the binary you get is `herd`.) Or from this repository:

```bash
cargo install --path .
```

## Feed it

**Claude Code** sessions self-report through hooks. Print the wiring and merge
it into `~/.claude/settings.json`:

```bash
herd hooks
```

That covers the whole lifecycle: prompt → `working`, tool calls → a throttled
heartbeat, a permission ask → `blocked`, stop → `done`, session end → gone.
The commands are `|| true` on purpose — a missing or broken herd must never
break a session.

**Agents under [herdr](https://herdr.dev)** that can't self-report (codex,
gemini, anything else herdr classifies) are mirrored in on demand:

```bash
herd sync-herdr
```

(Different tool, one letter apart, on purpose kept distinct: `herdr` is a
terminal workspace manager; `herd` is this bus. The verb reads literally —
"herd, sync from herdr".) Kinds that self-report are deliberately *not*
mirrored — the self report is the better source, and a duplicate entry is
worse than a missing one.

**Anything else** can just say so:

```bash
herd report --key build-42 --agent mybot --state blocked --title "needs a review"
```

## Drink from it

```bash
herd status          # the flock, attention-first
herd status --json   # same, with the staleness contract pre-applied
herd watch           # newline-delimited JSON on every change
```

Or read `~/.local/state/herd/state.json` directly — it is written with atomic
renames and is safe to watch. If you read the file yourself, you owe it one
rule:

> **An entry past its `stale_after` is `unknown`, whatever its `state` field
> still says.** Writers precompute it, so the whole contract is a single
> comparison — and an honest display renders `unknown` as *needs attention*,
> never as calm. A guess must not look like a fact.

States: `working` · `blocked` · `done` · `idle` · `error` · `unknown`.

**`done` is not `idle`.** Finished-and-unseen is the state an attention queue
exists for, and it ends only when a human looks:

```bash
herd seen <key>      # what a display calls after it focuses an agent
```

## Detection rules are data

One JSON file per agent kind in `~/.config/herd/manifests/` (see
`herd paths`): the kind's name, whether it self-reports, its `working`
heartbeat TTL, and the names herdr uses for it. Adding an agent kind is adding
a file, not compiling anything. Manifests are gated on an engine version so
rules can ship out-of-band from the binary.

## Scope, honestly

- Claude Code is the only agent that self-reports today; everything else
  arrives via the herdr mirror or `herd report`.
- There is no daemon. Verbs are short-lived; `watch` is a cheap poll.
- There is no screen-classification engine, and none is planned here.
- `herd wait --blocked` (block until someone genuinely needs a human) is
  designed but not yet built — tracked in this repository's issues.
- Sessions started before the hooks were installed appear on their next
  event.

## Lineage

This began life as **crook** (the yanked `crook` crate, versions 0.1.x) inside
a repository named addev, before the family found its true names: the bus is
the herd, the singling-out is the crook, the coat is the wool. The decision
records under `docs/decisions/` keep the original naming as history.

## What Herd is not

Not a multiplexer, not a UI, not a place where policy lives. It answers one
question — *who needs you* — and hands the answer to whatever is already on
your screen.

MIT.
