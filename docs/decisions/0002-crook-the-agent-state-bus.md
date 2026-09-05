# 0002 — Crook, the agent-state bus

Date: 2026-09-03
Status: accepted; v0.1 built 2026-09-05 (see the addendum at the end)

Depends on `0001 — herdr is read, not copied`.

## Context

`docs/vision.md` left three questions open. This record answers the second of
them — *what does a solo developer's attention surface look like?* — and the
answer is not the one the question implies.

Evidence gathered on 2026-09-03, in this developer's own stack:

- **Terminal Delight already has one.** `app/src/hud.rs` opens by describing
  itself as the read-only scoreboard over a wall of agent panes, for knowing at a
  glance who is working, who is blocked on you, and how much they have spent. It
  classifies each pane as working, blocked, idle or errored from the bottom row
  of the screen.
- **The Omarchy bar already has one.** `omarchy-herd`, published 2026-09-03,
  raises an icon and a tray the moment an agent is waiting on a human.
- **herdr already has one**, rolling pane state up through tab and workspace.

Three displays. The display is not the missing piece.

What is missing sits underneath all three. `is_blocked_prompt` in `hud.rs:158`
is eight hardcoded English substrings — `"do you want to proceed"`, `"(y/n)"`,
`"waiting for your"` and five more. The first time an agent vendor rewords its
permission prompt, that wall goes quiet, and it goes quiet *silently*, which is
the worst failure a status display has: it reports calm. Supporting a second
agent kind means writing more Rust. Meanwhile `omarchy-herd` gets the same
information from an entirely separate mechanism, and the two cannot agree or
disagree because nothing connects them.

Two facts about the shape of the fix, both established by reading herdr and both
usable under `0001 — herdr is read, not copied` as facts rather than expression:
per-agent detection belongs in data rather than compiled logic, and detection
rules have to be shippable without a binary release, because the thing they
describe changes in other people's patch releases. A third fact comes from
herdr's open issue list, where the recurring complaint is agents misreported as
idle mid-turn: screen classification is the weaker source, and an agent that can
be made to state its own status should be.

## Decision

ADDev's first component is **Crook** — a local agent-state bus. One writer, many
displays. It draws nothing.

- **Detection rules are data.** One file per agent kind under
  `~/.config/crook/manifests/`, not compiled logic. Adding an agent is adding a
  file, and a user can add one without a build toolchain.
- **Rules ship out-of-band from the binary**, gated on an engine version, so
  detection can follow agents that change between ADDev releases.
- **Self-reporting is primary, screen classification is the fallback.** Where an
  agent can be made to report its own state — Claude Code hooks, opencode's
  config — that path wins, and the published state says which source it came
  from so a consumer can weigh it.
- **`done` and `idle` are different states.** Finished-and-unseen is not
  finished-and-seen. An attention queue is worthless without that distinction,
  and it is the one most tools collapse.
- **It publishes to a well-known path** — `~/.local/state/crook/state.json` —
  and offers `crook status`, `crook watch`, and `crook wait --blocked`, the last
  so that a script or an agent can block until another agent is genuinely
  blocked rather than merely quiet.

Terminal Delight's wall becomes its first consumer, and `is_blocked_prompt` is
deleted when Crook lands. `omarchy-herd` becomes the second, or stays on herdr's
events and gains nothing, which is an acceptable outcome and a useful test.

## What Crook is not

It is not a multiplexer, and ADDev is not going to write one. herdr is 244,000
lines of first-party Rust across 1,511 commits, 85% of them from a single very
fast author. Competing there is a loss taken deliberately at the start. Crook
runs alongside herdr, alongside tmux, or alongside nothing at all — its input is
agents, not panes it owns.

It is also not a UI, a daemon with opinions about your layout, or a place where
policy lives. It answers one question — who needs you — and hands the answer to
whatever is already on your screen.

## The bet, stated so it can be lost

ADDev acquires a component before it has an environment. That is deliberate, and
it rests on the only demand signal this project has: a stranger opened a pull
request against `omarchy-herd` three hours after it appeared in herdr's plugin
marketplace, for a display fed by a state file. Nobody has asked for an agentic
development environment. Somebody, unprompted, improved the thing that says
which agent needs you.

Crook is the bet that the state file is the part worth generalising.

If, once it exists, nothing but Terminal Delight consumes it, that is the bet
losing and this record should be revisited rather than defended. The check is
concrete: a consumer that is neither Terminal Delight nor `omarchy-herd`, inside
six months of the first release.

## Deliberately unresolved

- What a manifest actually contains. herdr's answer is known and is not
  available to us; ours has to be arrived at from the problem.
- Whether Crook is a daemon, a library that hosts embed, or both.
- Whether the published state is a file, a Unix socket, or a file with a socket
  for change notification.
- Whether `crook wait` belongs in the first release at all, or is the second
  thing once the state is trusted.

## Naming

A crook is the shepherd's hook, the tool for singling one animal out of a flock
— which is the whole job. Free on crates.io as of 2026-09-03. Bellwether was the
first choice and is taken in this exact niche by `joelhooks/pi-bellwether`, a
herdr session manager.

## Addendum — what v0.1 resolved (2026-09-05)

Built overnight as `crook/` in this repository, forced by a consumer: the
`omarchy-agent-wool` wall was about to become the fourth parallel state
detector, which is the pathology this record diagnoses. Each deliberately
unresolved question got the smallest honest answer:

- **Daemon vs library:** neither. Short-lived verbs over one flock-locked,
  atomically renamed file. There is no process to babysit, and `watch` is a
  300ms mtime poll.
- **File vs socket:** a file, at the well-known path. Consumers may either
  read it directly — the whole staleness contract is one comparison, because
  writers precompute `stale_after` — or ask `crook status --json`, which also
  applies it.
- **`crook wait`:** deferred, as contemplated. Tracked as a follow-up issue.
- **What a manifest contains, v1 slice:** agent kind, whether it
  self-reports (in which case the herdr mirror skips it — no correlation
  problem, no duplicate entries), the `working` heartbeat TTL, and the names
  herdr uses for the kind. JSON, one file per kind, engine-gated. The full
  screen-classification engine stays deliberately absent.
- **Self-reporting, realised:** machine-global Claude Code hooks call
  `crook hook` on SessionStart, UserPromptSubmit, PostToolUse (throttled
  heartbeat), Notification, Stop and SessionEnd. Stop maps to `done` and a
  display calling `crook seen` is what ends it — finished-and-unseen is a
  first-class state, as demanded above.
- **Liveness:** entries carry the agent's pid where discoverable; a dead pid
  past a grace window is pruned, a stale `working` decays to `unknown`, and
  unknown draws as attention, never as calm.

Verified live the night it was built: a probe session's idle → working → done
lifecycle captured off the file, three concurrent real sessions self-reporting
within minutes, herdr's claude panes correctly not mirrored.

The bet's clock now runs from tonight: Wool is a consumer that is neither
Terminal Delight nor `omarchy-herd`, but it is still this developer's own
software — external adoption remains the signal that wins the bet.

Later the same day, on releasing publicly (addev public, crook 0.1.0 on
crates.io), Parker withdrew the six-month framing outright — it had already
been misused once, as an argument for delaying the release it was supposed to
evaluate. External adoption stays interesting as a signal; it is not a clock,
not a gate, and no future decision should cite it as either.
