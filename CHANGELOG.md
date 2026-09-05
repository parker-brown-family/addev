# Changelog

## 0.2.0 — 2026-09-05

First release under the name Herd, and the first one published as a crate.

- Renamed from **crook** to **Herd**. The bus is the flock's state, which is
  what a herd is; the name crook had accidentally described the bar tray that
  singles one agent out of it, and that tray now carries it. The binary is
  `herd`, its verbs are `herd report`, `herd sync-herdr`, `herd seen` and
  `herd watch`, and the state file is `~/.local/state/herd/state.json`.
- Promoted out of the `crook/` subdirectory of a repository named addev to the
  root of a repository named herd. `cargo install --path .` works, CI needs no
  `--manifest-path`, and the umbrella name that described nothing anyone
  shipped is gone. `docs/addev-seed-readme.md` keeps the original README.
- Published as the `herd-bus` crate, because crates.io's `herd` is a squat
  from 2022. The binary it installs is still `herd`. The old `crook` crate
  (0.1.x) is yanked.

## 0.1.1 — 2026-09-04

- Injected machinery no longer masquerades as the human's prompt: a hook-fed
  turn is recorded as what it is, so a session waiting on a person is
  distinguishable from one waiting on a script.

## 0.1.0 — 2026-09-03

First working bus, then named crook.

- Agents report their own state into one file; every display reads it. One
  writer, many readers, atomic renames, no daemon.
- Claude Code hook wiring, a herdr mirror, and a private state directory.
