# Handoff — Crook v0.1: the agent-state bus exists and the fleet is on it

**Date:** 2026-09-05 (overnight build)
**Status:** built, tested, installed, wired, verified live
**Where:** `crook/` in this repo; binary at `~/.local/bin/crook`

The decision record (`docs/decisions/0002`, "Crook, the agent-state bus") went
from *accepted, unbuilt* to *v0.1 built* — its new addendum records which of
the deliberately-unresolved questions got answered and how. Forced by a
consumer: the `omarchy-agent-wool` wall was one ticket away from becoming the
fourth parallel state detector.

## Who feeds it, who drinks

- **Feed:** Claude Code hooks (machine-global in `~/.claude/settings.json`:
  SessionStart, UserPromptSubmit, PostToolUse, Notification, Stop, SessionEnd
  → `crook hook`, absolute path, `|| true`, 5s timeout); `crook report` for
  anything else; `crook sync-herdr` mirrors non-self-reporting agents (claude
  panes under herdr are deliberately skipped — the self report wins and no
  correlation is attempted).
- **Drink:** `~/.local/state/crook/state.json` (atomic renames, safe to
  watch), or `crook status --json` / `crook watch`. Wool is the first
  consumer.
- **The one back-signal:** `crook seen <key>` — a display saying a human
  looked; flips done → idle.

## Contracts a future session must not break

- **Readers:** past `stale_after`, an entry is UNKNOWN whatever `state` says.
  Writers precompute it (working TTL 900s from manifest; herdr entries 60s;
  blocked/done/idle never decay).
- **`crook hook` prints NOTHING on stdout, ever, and always exits 0** —
  UserPromptSubmit/SessionStart hook stdout is injected into conversations,
  and a non-zero exit can block one. This is load-bearing, not style.
- **File writes:** flock + write-temp + rename, always. One writer at a time.
- **PostToolUse is throttled** (20s same-state window) — the heartbeat is
  wanted, a write per tool call is not.

## Verified

- `cargo test` — 12/12 (state transitions, TTLs, throttle, prune, hook
  mapping, herdr mirroring/skip/removal).
- Live: probe `claude -p` session's idle → working (title = the prompt) →
  done lifecycle captured from the file; three concurrent real sessions
  self-reported within minutes; dead-pid prune cleaned the probe up after its
  grace window.
- Real herdr: its two claude panes correctly not mirrored.

## How to run / verify again

```bash
cargo test --manifest-path /home/parker/Work/addev/crook/Cargo.toml
```
```bash
crook status --json | jq .
```
```bash
crook watch
```

## Not done / next

- `crook wait --blocked` — deferred by the record; filed as a follow-up issue.
- Screen-classification manifests (the herdr-shaped half) — deliberately
  absent from v1; the manifest carries only kind, self_reporting, TTL, herdr
  names.
- Terminal Delight consuming crook and deleting `is_blocked_prompt`
  (hud.rs's eight hardcoded English substrings) — filed on terminal-delight.
- A `crook drop <key>` verb would be handy for hygiene (tonight's smoke used
  a synthetic SessionEnd instead). Minor.
- Sessions started before the hooks existed stay invisible until their next
  event — cold-start gap, self-heals, worth knowing when the wall looks
  sparse right after install.

## Watch out

- The hook envelope fields used: `session_id`, `hook_event_name`, `cwd`,
  `transcript_path`, `prompt`, `message`. Anything Claude adds is ignored;
  an unknown `hook_event_name` is a deliberate no-op, never a guess.
- pid discovery climbs /proc from the hook process looking for `claude`;
  when it fails the entry has `pid: null` and only ages out — unknown is not
  zero, so no pid is *not* treated as dead.
