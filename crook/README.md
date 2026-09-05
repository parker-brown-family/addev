# crook

**A local agent-state bus.** One writer, many displays; it draws nothing.

You run coding agents. Some are working, one finished ten minutes ago and is
waiting for you to look, one is sitting on a permission prompt, and one hung.
Every tool that shows you this today detects it separately — usually by
matching phrases on a screen — so the displays can't agree, and the first time
an agent vendor rewords a prompt, your status surface goes quiet *and reports
calm*.

crook is the layer underneath: agents report their own state into one file,
and everything on your screen reads that file.

```
feed:  Claude Code hooks ─┐
       crook report ──────┼──→  ~/.local/state/crook/state.json  ──→  read: your bar widget,
       crook sync-herdr ──┘        (atomic renames, safe to watch)         wall, tmux line,
                                                                           crook watch scripts
```

Known consumers:
[Wool](https://github.com/parker-brown-family/omarchy-agent-wool) (an Omarchy
wall of every agent) — with [Herd](https://github.com/parker-brown-family/omarchy-herd)
and Terminal Delight's agent wall as intended next.

## Install

```bash
cargo install crook
```

or from this repository:

```bash
cargo install --path crook
```

## Feed it

**Claude Code** sessions self-report through hooks. Print the wiring and merge
it into `~/.claude/settings.json`:

```bash
crook hooks
```

That covers the whole lifecycle: prompt → `working`, tool calls → a throttled
heartbeat, a permission ask → `blocked`, stop → `done`, session end → gone.
The commands are `|| true` on purpose — a missing or broken crook must never
break a session.

**Agents under [herdr](https://herdr.dev)** that can't self-report (codex,
gemini, anything else herdr classifies) are mirrored in on demand:

```bash
crook sync-herdr
```

Kinds that self-report are deliberately *not* mirrored — the self report is
the better source, and a duplicate entry is worse than a missing one.

**Anything else** can just say so:

```bash
crook report --key build-42 --agent mybot --state blocked --title "needs a review"
```

## Drink from it

```bash
crook status          # the flock, attention-first
crook status --json   # same, with the staleness contract pre-applied
crook watch           # newline-delimited JSON on every change
```

Or read `~/.local/state/crook/state.json` directly — it is written with
atomic renames and is safe to watch. If you read the file yourself, you owe
it one rule:

> **An entry past its `stale_after` is `unknown`, whatever its `state` field
> still says.** Writers precompute it, so the whole contract is a single
> comparison — and an honest display renders `unknown` as *needs attention*,
> never as calm. A guess must not look like a fact.

States: `working` · `blocked` · `done` · `idle` · `error` · `unknown`.

**`done` is not `idle`.** Finished-and-unseen is the state an attention queue
exists for, and it ends only when a human looks:

```bash
crook seen <key>      # what a display calls after it focuses an agent
```

## Detection rules are data

One JSON file per agent kind in `~/.config/crook/manifests/` (see
`crook paths`): the kind's name, whether it self-reports, its `working`
heartbeat TTL, and the names herdr uses for it. Adding an agent kind is
adding a file, not compiling anything. Manifests are gated on an engine
version so rules can ship out-of-band from the binary.

## Scope, honestly

- Claude Code is the only agent that self-reports today; everything else
  arrives via the herdr mirror or `crook report`.
- There is no daemon. Verbs are short-lived; `watch` is a cheap poll.
- There is no screen-classification engine, and none is planned here.
- `crook wait --blocked` (block until someone genuinely needs a human) is
  designed but not yet built — tracked in this repository's issues.
- Sessions started before the hooks were installed appear on their next
  event.

## What crook is not

Not a multiplexer, not a UI, not a place where policy lives. It answers one
question — *who needs you* — and hands the answer to whatever is already on
your screen.

Part of [ADDev](../README.md). MIT.
