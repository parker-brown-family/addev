// Crook — the agent-state bus. One writer, many displays; it draws nothing.
//
// Everything here is a short-lived verb over one file:
//
//   ${XDG_STATE_HOME:-~/.local/state}/crook/state.json
//
// There is deliberately no daemon. Agents feed the file (Claude Code hooks call
// `crook hook`; anything else can call `crook report`); `crook sync-herdr`
// mirrors in the agents that cannot speak for themselves; displays read the
// file. The write path is flock + write-temp + rename, so a reader never sees
// a half-written document and two writers never interleave.
//
// The one rule consumers must honour: an entry whose `stale_after` has passed
// is UNKNOWN, not whatever its `state` field still says. A status surface that
// renders a stale `working` as calm is reporting a guess as a fact — the
// failure this program exists to end. Writers precompute `stale_after` so the
// whole contract is a single comparison on the read side.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

const ENGINE: u32 = 1;
const VERSION: &str = env!("CARGO_PKG_VERSION");

// A same-state re-report inside this window is dropped. PostToolUse fires on
// every tool call an agent makes; the heartbeat is wanted, one write per call
// is not.
const THROTTLE_SECS: u64 = 20;

// A dead pid is proof the session is gone, but the pid is read moments after
// the report that carried it — give the scheduler a beat before believing it.
const DEAD_PID_GRACE_SECS: u64 = 60;

// An entry that never carried a pid can only age out. A week is long enough
// that nothing real survives it, short enough that the file cannot grow
// without bound.
const NO_PID_EXPIRY_SECS: u64 = 7 * 24 * 3600;

// herdr-sourced entries are only as fresh as the last sync, and syncs are
// driven by whoever is looking (a wall tick, a status call). Anything older
// than this is a guess and must read as one.
const HERDR_TTL_SECS: u64 = 60;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------- documents

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Meta {
    version: String,
    engine: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Entry {
    /// Claude session uuid, or "herdr:<pane_id>", or whatever a reporter picks.
    key: String,
    /// Agent kind: "claude-code", "codex", ... Manifest vocabulary.
    agent: String,
    /// working | blocked | done | idle | error | unknown
    state: String,
    /// Who said so: "self" outranks "herdr" outranks anything screen-derived.
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transcript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    updated_at: u64,
    /// Past this instant the entry is UNKNOWN to every honest reader.
    /// None means the state does not decay (blocked stays blocked until
    /// answered; done stays done until seen).
    #[serde(skip_serializing_if = "Option::is_none")]
    stale_after: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Doc {
    crook: Meta,
    updated_at: u64,
    sessions: Vec<Entry>,
}

impl Doc {
    fn empty() -> Self {
        Doc {
            crook: Meta { version: VERSION.into(), engine: ENGINE },
            updated_at: now(),
            sessions: Vec::new(),
        }
    }
    fn find_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.sessions.iter_mut().find(|e| e.key == key)
    }
}

// ---------------------------------------------------------------- manifests

// Detection rules are data, not compiled logic — the record's first
// load-bearing choice. v1 carries the smallest honest slice of that: what an
// agent kind is called, whether it reports its own state (in which case the
// herdr mirror must not double it), and how fast a `working` claim rots.
// The full screen-classification engine herdr runs is deliberately absent.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Manifest {
    engine: u32,
    agent: String,
    #[serde(default)]
    self_reporting: bool,
    /// Seconds a self-reported `working` stays believable without a heartbeat.
    #[serde(default)]
    working_ttl: Option<u64>,
    /// Names herdr uses for this kind (`agent` / `display_agent` fields).
    #[serde(default)]
    herdr_names: Vec<String>,
}

fn builtin_manifests() -> Vec<Manifest> {
    vec![
        Manifest {
            engine: 1,
            agent: "claude-code".into(),
            self_reporting: true,
            // Hooks heartbeat on every tool call; a quarter hour of silence
            // from a "working" agent is a hang, and a hang must not render
            // as work.
            working_ttl: Some(900),
            herdr_names: vec!["claude".into(), "Claude Code".into()],
        },
        Manifest {
            engine: 1,
            agent: "codex".into(),
            self_reporting: false,
            working_ttl: None,
            herdr_names: vec!["codex".into(), "Codex".into()],
        },
        Manifest {
            engine: 1,
            agent: "gemini".into(),
            self_reporting: false,
            working_ttl: None,
            herdr_names: vec!["gemini".into(), "Gemini".into()],
        },
    ]
}

fn load_manifests(paths: &Paths) -> Vec<Manifest> {
    let mut by_agent: BTreeMap<String, Manifest> = BTreeMap::new();
    for m in builtin_manifests() {
        by_agent.insert(m.agent.clone(), m);
    }
    if let Ok(entries) = std::fs::read_dir(&paths.manifest_dir) {
        for f in entries.flatten() {
            let p = f.path();
            if p.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str::<Manifest>(&s).ok()) {
                Some(m) if m.engine <= ENGINE => {
                    by_agent.insert(m.agent.clone(), m);
                }
                Some(m) => {
                    eprintln!("crook: manifest {} wants engine {} (this crook speaks {}); skipped", p.display(), m.engine, ENGINE);
                }
                None => {
                    eprintln!("crook: manifest {} is not readable as a manifest; skipped", p.display());
                }
            }
        }
    }
    by_agent.into_values().collect()
}

fn manifest_for<'a>(manifests: &'a [Manifest], agent: &str) -> Option<&'a Manifest> {
    manifests.iter().find(|m| m.agent == agent)
}

fn kind_for_herdr_name<'a>(manifests: &'a [Manifest], name: &str) -> Option<&'a Manifest> {
    manifests
        .iter()
        .find(|m| m.herdr_names.iter().any(|n| n.eq_ignore_ascii_case(name)))
}

// -------------------------------------------------------------------- paths

struct Paths {
    state_dir: PathBuf,
    manifest_dir: PathBuf,
}

impl Paths {
    fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
        let state_base = std::env::var("XDG_STATE_HOME")
            .unwrap_or_else(|_| format!("{home}/.local/state"));
        let config_base = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or_else(|_| format!("{home}/.config"));
        Paths {
            state_dir: PathBuf::from(state_base).join("crook"),
            manifest_dir: PathBuf::from(config_base).join("crook/manifests"),
        }
    }
    fn state_file(&self) -> PathBuf {
        self.state_dir.join("state.json")
    }
    fn lock_file(&self) -> PathBuf {
        self.state_dir.join(".lock")
    }
}

// ------------------------------------------------------------------ locking

struct Lock {
    _file: std::fs::File,
}

fn lock(paths: &Paths) -> std::io::Result<Lock> {
    std::fs::create_dir_all(&paths.state_dir)?;
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(paths.lock_file())?;
    let rc = unsafe { libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&f), libc::LOCK_EX) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(Lock { _file: f })
}

fn load_doc(paths: &Paths) -> Doc {
    match std::fs::read_to_string(paths.state_file()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| Doc::empty()),
        Err(_) => Doc::empty(),
    }
}

fn save_doc(paths: &Paths, doc: &mut Doc) -> std::io::Result<()> {
    doc.updated_at = now();
    doc.crook = Meta { version: VERSION.into(), engine: ENGINE };
    // Attention order in the file itself, so the laziest possible consumer —
    // one that just draws the array — still puts the needful first.
    doc.sessions.sort_by_key(|e| (state_rank(&e.state), e.key.clone()));
    let tmp = paths.state_dir.join(format!(".state.json.tmp.{}", std::process::id()));
    let body = serde_json::to_string_pretty(doc).expect("state serializes");
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, paths.state_file())?;
    Ok(())
}

fn state_rank(state: &str) -> u8 {
    match state {
        "blocked" => 0,
        "error" => 1,
        "done" => 2,
        "working" => 3,
        "unknown" => 4,
        "idle" => 5,
        _ => 6,
    }
}

fn valid_state(s: &str) -> bool {
    matches!(s, "working" | "blocked" | "done" | "idle" | "error" | "unknown")
}

// ------------------------------------------------------------------- report

struct Report {
    key: String,
    agent: String,
    state: String,
    source: String,
    title: Option<String>,
    cwd: Option<String>,
    pid: Option<i32>,
    transcript: Option<String>,
    detail: Option<String>,
}

fn stale_after_for(manifests: &[Manifest], agent: &str, state: &str, at: u64) -> Option<u64> {
    if state == "working" {
        manifest_for(manifests, agent)
            .and_then(|m| m.working_ttl)
            .map(|ttl| at + ttl)
    } else {
        None
    }
}

/// Returns true when the document changed.
fn apply_report(doc: &mut Doc, manifests: &[Manifest], r: Report, at: u64) -> bool {
    let stale_after = stale_after_for(manifests, &r.agent, &r.state, at);
    if let Some(e) = doc.find_mut(&r.key) {
        let same = e.state == r.state && e.source == r.source && e.detail == r.detail;
        if same && at.saturating_sub(e.updated_at) < THROTTLE_SECS {
            // Refreshing stale_after is the entire value of a heartbeat, but
            // rewriting the file for every tool call is not; inside the
            // throttle window the previous stamp is close enough.
            return false;
        }
        e.agent = r.agent;
        e.state = r.state;
        e.source = r.source;
        if r.title.is_some() {
            e.title = r.title;
        }
        if r.cwd.is_some() {
            e.cwd = r.cwd;
        }
        if r.pid.is_some() {
            e.pid = r.pid;
        }
        if r.transcript.is_some() {
            e.transcript = r.transcript;
        }
        e.detail = r.detail;
        e.updated_at = at;
        e.stale_after = stale_after;
        return true;
    }
    doc.sessions.push(Entry {
        key: r.key,
        agent: r.agent,
        state: r.state,
        source: r.source,
        title: r.title,
        cwd: r.cwd,
        pid: r.pid,
        transcript: r.transcript,
        detail: r.detail,
        updated_at: at,
        stale_after,
    });
    true
}

fn remove_key(doc: &mut Doc, key: &str) -> bool {
    let before = doc.sessions.len();
    doc.sessions.retain(|e| e.key != key);
    doc.sessions.len() != before
}

// -------------------------------------------------------------------- prune

fn pid_alive(pid: i32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

/// Drop entries whose process is provably gone, or that never named a pid and
/// have aged past belief. Returns true when anything was dropped.
fn prune(doc: &mut Doc, at: u64) -> bool {
    let before = doc.sessions.len();
    doc.sessions.retain(|e| {
        let age = at.saturating_sub(e.updated_at);
        match e.pid {
            Some(pid) => pid_alive(pid) || age < DEAD_PID_GRACE_SECS,
            None => age < NO_PID_EXPIRY_SECS,
        }
    });
    doc.sessions.len() != before
}

/// The one-line staleness contract, applied for consumers who ask crook
/// rather than reading the file themselves.
fn effective_state(e: &Entry, at: u64) -> &str {
    match e.stale_after {
        Some(t) if at > t => "unknown",
        _ => e.state.as_str(),
    }
}

// --------------------------------------------------------------------- hook

// Claude Code hook envelopes. Only the fields used here are named; the
// envelope is Claude's to grow.
#[derive(Deserialize)]
struct HookPayload {
    session_id: Option<String>,
    transcript_path: Option<String>,
    cwd: Option<String>,
    hook_event_name: Option<String>,
    prompt: Option<String>,
    message: Option<String>,
}

/// One line, bounded, no control characters — this string ends up in a UI.
fn clean_line(s: &str, max: usize) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if out.chars().count() >= max {
            out.push('…');
            break;
        }
        out.push(if ch.is_control() { ' ' } else { ch });
    }
    out.trim().to_string()
}

/// Climb from this process toward init looking for the Claude Code process
/// itself. The hook command runs as claude → sh → crook, but the number of
/// intermediate shells is not ours to assume.
fn find_claude_ancestor() -> Option<i32> {
    let mut pid = unsafe { libc::getppid() };
    for _ in 0..12 {
        if pid <= 1 {
            return None;
        }
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        let cmdline = std::fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        if comm.trim() == "claude" || cmdline.split('\0').next().is_some_and(|a| a.ends_with("/claude") || a == "claude") {
            return Some(pid);
        }
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        // field 4 of /proc/pid/stat is ppid; comm (field 2) may contain
        // spaces but is parenthesised, so split after the closing paren.
        let ppid = stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().nth(1))
            .and_then(|s| s.parse::<i32>().ok());
        match ppid {
            Some(p) => pid = p,
            None => return None,
        }
    }
    None
}

fn hook_event_to_report(payload: &HookPayload) -> Option<Report> {
    let key = payload.session_id.clone()?;
    let event = payload.hook_event_name.as_deref()?;
    let state = match event {
        "SessionStart" => "idle",
        "UserPromptSubmit" => "working",
        "PreToolUse" | "PostToolUse" => "working",
        "Notification" => "blocked",
        "Stop" => "done",
        // SessionEnd is handled as a removal before this function is called;
        // any event Claude invents later is deliberately a no-op rather than
        // a guess.
        _ => return None,
    };
    Some(Report {
        key,
        agent: "claude-code".into(),
        state: state.into(),
        source: "self".into(),
        title: payload.prompt.as_deref().map(|p| clean_line(p, 120)),
        cwd: payload.cwd.clone(),
        pid: None, // filled by the caller, which knows it is inside a hook
        transcript: payload.transcript_path.clone(),
        detail: match event {
            "Notification" => payload.message.as_deref().map(|m| clean_line(m, 160)),
            _ => None,
        },
    })
}

fn cmd_hook(paths: &Paths) -> i32 {
    // Nothing this verb does may break or pollute a Claude session: stdout
    // from several hook events is injected into the conversation, and a
    // non-zero exit can block one. Silence and success, no matter what.
    let mut body = String::new();
    if std::io::stdin().read_to_string(&mut body).is_err() {
        return 0;
    }
    let payload: HookPayload = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(_) => return 0,
    };
    let manifests = load_manifests(paths);
    let Ok(_guard) = lock(paths) else { return 0 };
    let mut doc = load_doc(paths);
    let at = now();

    let changed = if payload.hook_event_name.as_deref() == Some("SessionEnd") {
        match &payload.session_id {
            Some(key) => remove_key(&mut doc, key),
            None => false,
        }
    } else {
        match hook_event_to_report(&payload) {
            Some(mut r) => {
                r.pid = find_claude_ancestor();
                apply_report(&mut doc, &manifests, r, at)
            }
            None => false,
        }
    };

    if changed {
        let _ = save_doc(paths, &mut doc);
    }
    0
}

// -------------------------------------------------------------- sync-herdr

#[derive(Deserialize)]
struct HerdrList {
    result: Option<HerdrResult>,
}
#[derive(Deserialize)]
struct HerdrResult {
    #[serde(default)]
    agents: Vec<HerdrAgent>,
}
#[derive(Deserialize)]
struct HerdrAgent {
    pane_id: Option<String>,
    agent: Option<String>,
    display_agent: Option<String>,
    agent_status: Option<String>,
    cwd: Option<String>,
    name: Option<String>,
    terminal_title_stripped: Option<String>,
}

fn map_herdr_state(s: &str) -> &str {
    // herdr's vocabulary is interoperated with by name (0001: an interface you
    // must match to talk to a program is a fact about it). It has no error
    // state — failures fold into unknown — so nothing here invents one.
    match s {
        "working" | "blocked" | "done" | "idle" => s,
        _ => "unknown",
    }
}

/// Fold a herdr agent list (the raw JSON of `herdr agent list`) into the doc.
/// Returns true when the document changed.
fn apply_herdr_list(doc: &mut Doc, manifests: &[Manifest], raw: &str, at: u64) -> bool {
    let parsed: Option<HerdrList> = serde_json::from_str(raw).ok();
    let agents = parsed
        .and_then(|l| l.result)
        .map(|r| r.agents)
        .unwrap_or_default();

    let mut seen_keys: Vec<String> = Vec::new();
    let mut changed = false;

    for a in agents {
        let Some(pane) = a.pane_id.clone() else { continue };
        let herdr_name = a
            .display_agent
            .clone()
            .or(a.agent.clone())
            .unwrap_or_else(|| "agent".into());
        // Kinds that report their own state are not mirrored: the self report
        // is the better source, and a second entry for the same agent under a
        // different key is worse than a missing one.
        if let Some(m) = kind_for_herdr_name(manifests, &herdr_name) {
            if m.self_reporting {
                continue;
            }
        }
        let key = format!("herdr:{pane}");
        seen_keys.push(key.clone());
        let kind = kind_for_herdr_name(manifests, &herdr_name)
            .map(|m| m.agent.clone())
            .unwrap_or(herdr_name);
        let state = map_herdr_state(a.agent_status.as_deref().unwrap_or("unknown"));
        let title = match (a.name.as_deref(), a.terminal_title_stripped.as_deref()) {
            (Some(n), Some(t)) if !t.is_empty() => Some(format!("{n} · {t}")),
            (Some(n), _) => Some(n.to_string()),
            (None, Some(t)) if !t.is_empty() => Some(t.to_string()),
            _ => None,
        };
        let r = Report {
            key,
            agent: kind,
            state: state.into(),
            source: "herdr".into(),
            title,
            cwd: a.cwd.clone(),
            pid: None,
            transcript: None,
            detail: None,
        };
        // herdr entries rot fast by design: they are only as fresh as the
        // last sync, and the sync runs only while something is looking.
        changed |= apply_report(doc, manifests, r, at);
    }

    // Stamp the herdr TTL on everything herdr-sourced, and drop the mirrored
    // entries herdr no longer lists — the server is the truth for its own set.
    for e in doc.sessions.iter_mut() {
        if e.source == "herdr" && seen_keys.contains(&e.key) {
            if e.stale_after != Some(at + HERDR_TTL_SECS) {
                e.stale_after = Some(at + HERDR_TTL_SECS);
                changed = true;
            }
        }
    }
    let before = doc.sessions.len();
    doc.sessions
        .retain(|e| e.source != "herdr" || seen_keys.contains(&e.key));
    changed |= doc.sessions.len() != before;
    changed
}

fn cmd_sync_herdr(paths: &Paths) -> i32 {
    let out = std::process::Command::new("herdr")
        .args(["agent", "list"])
        .output();
    let raw = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        // No herdr, or no server: an empty list is the honest mirror, and
        // apply_herdr_list on an empty document drops every herdr entry.
        _ => String::new(),
    };
    let manifests = load_manifests(paths);
    let Ok(_guard) = lock(paths) else { return 1 };
    let mut doc = load_doc(paths);
    let at = now();
    let mut changed = apply_herdr_list(&mut doc, &manifests, &raw, at);
    changed |= prune(&mut doc, at);
    if changed {
        if let Err(e) = save_doc(paths, &mut doc) {
            eprintln!("crook: cannot write state: {e}");
            return 1;
        }
    }
    0
}

// ------------------------------------------------------------------- verbs

fn cmd_report(paths: &Paths, args: &[String]) -> i32 {
    let mut key = None;
    let mut agent = None;
    let mut state = None;
    let mut source = "manual".to_string();
    let mut title = None;
    let mut cwd = None;
    let mut pid = None;
    let mut transcript = None;
    let mut detail = None;

    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut grab = |name: &str| -> Option<String> {
            let v = it.next().cloned();
            if v.is_none() {
                eprintln!("crook report: {name} needs a value");
            }
            v
        };
        match a.as_str() {
            "--key" => key = grab("--key"),
            "--agent" => agent = grab("--agent"),
            "--state" => state = grab("--state"),
            "--source" => {
                if let Some(v) = grab("--source") {
                    source = v;
                }
            }
            "--title" => title = grab("--title"),
            "--cwd" => cwd = grab("--cwd"),
            "--pid" => pid = grab("--pid").and_then(|v| v.parse().ok()),
            "--transcript" => transcript = grab("--transcript"),
            "--detail" => detail = grab("--detail"),
            other => {
                eprintln!("crook report: unknown flag {other}");
                return 2;
            }
        }
    }

    let (Some(key), Some(agent), Some(state)) = (key, agent, state) else {
        eprintln!("crook report: --key, --agent and --state are required");
        return 2;
    };
    if !valid_state(&state) {
        eprintln!("crook report: state must be one of working|blocked|done|idle|error|unknown");
        return 2;
    }

    let manifests = load_manifests(paths);
    let Ok(_guard) = lock(paths) else {
        eprintln!("crook: cannot take the state lock");
        return 1;
    };
    let mut doc = load_doc(paths);
    let at = now();
    let changed = apply_report(
        &mut doc,
        &manifests,
        Report { key, agent, state, source, title, cwd, pid, transcript, detail },
        at,
    );
    if changed {
        if let Err(e) = save_doc(paths, &mut doc) {
            eprintln!("crook: cannot write state: {e}");
            return 1;
        }
    }
    0
}

fn cmd_seen(paths: &Paths, args: &[String]) -> i32 {
    let all = args.iter().any(|a| a == "--all");
    let key = args.iter().find(|a| !a.starts_with("--")).cloned();
    if !all && key.is_none() {
        eprintln!("crook seen: pass a session key, or --all");
        return 2;
    }
    let Ok(_guard) = lock(paths) else { return 1 };
    let mut doc = load_doc(paths);
    let at = now();
    let mut changed = false;
    for e in doc.sessions.iter_mut() {
        let matches = all || Some(&e.key) == key.as_ref();
        if matches && e.state == "done" {
            e.state = "idle".into();
            e.updated_at = at;
            e.stale_after = None;
            changed = true;
        }
    }
    if changed {
        let _ = save_doc(paths, &mut doc);
    }
    0
}

/// Status output entry: the raw record plus the staleness contract already
/// applied, for consumers who would rather ask than compute.
#[derive(Serialize)]
struct StatusEntry {
    #[serde(flatten)]
    entry: Entry,
    effective: String,
    age_seconds: u64,
}

fn cmd_status(paths: &Paths, json: bool) -> i32 {
    let Ok(_guard) = lock(paths) else { return 1 };
    let mut doc = load_doc(paths);
    let at = now();
    if prune(&mut doc, at) {
        let _ = save_doc(paths, &mut doc);
    }
    let rows: Vec<StatusEntry> = doc
        .sessions
        .iter()
        .map(|e| StatusEntry {
            entry: e.clone(),
            effective: effective_state(e, at).to_string(),
            age_seconds: at.saturating_sub(e.updated_at),
        })
        .collect();
    if json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "crook": doc.crook,
            "updated_at": doc.updated_at,
            "sessions": rows,
        })).unwrap());
        return 0;
    }
    if rows.is_empty() {
        println!("no agents reported. (hooks feed claude sessions; `crook sync-herdr` mirrors herdr)");
        return 0;
    }
    for r in rows {
        let title = r.entry.title.as_deref().unwrap_or("");
        let cwd = r
            .entry
            .cwd
            .as_deref()
            .map(|c| c.rsplit('/').next().unwrap_or(c))
            .unwrap_or("");
        println!(
            "{:<8} {:<12} {:<7} {:>4}s  {:<18} {}",
            r.effective,
            r.entry.agent,
            r.entry.source,
            r.age_seconds,
            cwd,
            title
        );
    }
    0
}

fn cmd_watch(paths: &Paths) -> i32 {
    // A poll on mtime, not inotify: one stat every 300ms is nothing, needs no
    // dependency, and cannot miss a rename the way a watch on the old inode
    // can. Emits one JSON document per change, newline-delimited.
    let mut last = None;
    loop {
        let mtime = std::fs::metadata(paths.state_file())
            .and_then(|m| m.modified())
            .ok();
        if mtime != last {
            last = mtime;
            let doc = load_doc(paths);
            let at = now();
            let rows: Vec<StatusEntry> = doc
                .sessions
                .iter()
                .map(|e| StatusEntry {
                    entry: e.clone(),
                    effective: effective_state(e, at).to_string(),
                    age_seconds: at.saturating_sub(e.updated_at),
                })
                .collect();
            let line = serde_json::to_string(&serde_json::json!({
                "updated_at": doc.updated_at,
                "sessions": rows,
            }))
            .unwrap();
            println!("{line}");
            let _ = std::io::stdout().flush();
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

fn cmd_paths(paths: &Paths) -> i32 {
    println!("state:     {}", paths.state_file().display());
    println!("manifests: {}", paths.manifest_dir.display());
    0
}

fn usage() -> i32 {
    eprintln!(
        "crook {VERSION} — local agent-state bus. One writer, many displays; draws nothing.

USAGE
  crook status [--json]     what the flock is doing (prunes dead sessions)
  crook watch               newline-delimited JSON on every state change
  crook report --key K --agent KIND --state S [--source --title --cwd --pid --transcript --detail]
  crook seen KEY|--all      done -> idle (finished-and-seen is not finished-and-unseen)
  crook sync-herdr          mirror herdr's agents (skips kinds that self-report)
  crook hook                Claude Code hook sink: reads the hook JSON on stdin
  crook paths               where state and manifests live

STATES  working blocked done idle error unknown
READERS An entry past its stale_after is UNKNOWN, whatever its state says.
FILE    ~/.local/state/crook/state.json (atomic renames; safe to watch)"
    );
    2
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths = Paths::from_env();
    let code = match args.first().map(|s| s.as_str()) {
        Some("status") => cmd_status(&paths, args.iter().any(|a| a == "--json")),
        Some("watch") => cmd_watch(&paths),
        Some("report") => cmd_report(&paths, &args[1..]),
        Some("seen") => cmd_seen(&paths, &args[1..]),
        Some("sync-herdr") => cmd_sync_herdr(&paths),
        Some("hook") => cmd_hook(&paths),
        Some("paths") => cmd_paths(&paths),
        Some("--version") | Some("-V") => {
            println!("crook {VERSION} (engine {ENGINE})");
            0
        }
        _ => usage(),
    };
    std::process::exit(code);
}

// -------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn manifests() -> Vec<Manifest> {
        builtin_manifests()
    }

    fn report(key: &str, state: &str) -> Report {
        Report {
            key: key.into(),
            agent: "claude-code".into(),
            state: state.into(),
            source: "self".into(),
            title: None,
            cwd: None,
            pid: None,
            transcript: None,
            detail: None,
        }
    }

    #[test]
    fn report_inserts_then_updates() {
        let mut doc = Doc::empty();
        assert!(apply_report(&mut doc, &manifests(), report("s1", "working"), 1000));
        assert_eq!(doc.sessions.len(), 1);
        assert!(apply_report(&mut doc, &manifests(), report("s1", "done"), 1100));
        assert_eq!(doc.sessions.len(), 1);
        assert_eq!(doc.sessions[0].state, "done");
    }

    #[test]
    fn same_state_reports_throttle() {
        let mut doc = Doc::empty();
        assert!(apply_report(&mut doc, &manifests(), report("s1", "working"), 1000));
        assert!(!apply_report(&mut doc, &manifests(), report("s1", "working"), 1005));
        // Past the window the heartbeat lands and refreshes stale_after.
        assert!(apply_report(&mut doc, &manifests(), report("s1", "working"), 1000 + THROTTLE_SECS + 1));
    }

    #[test]
    fn working_gets_a_ttl_and_it_reads_as_unknown_after() {
        let mut doc = Doc::empty();
        apply_report(&mut doc, &manifests(), report("s1", "working"), 1000);
        let e = &doc.sessions[0];
        let ttl = manifests()[0].working_ttl.unwrap();
        assert_eq!(e.stale_after, Some(1000 + ttl));
        assert_eq!(effective_state(e, 1000 + ttl - 1), "working");
        assert_eq!(effective_state(e, 1000 + ttl + 1), "unknown");
    }

    #[test]
    fn blocked_and_done_do_not_decay() {
        let mut doc = Doc::empty();
        apply_report(&mut doc, &manifests(), report("s1", "blocked"), 1000);
        apply_report(&mut doc, &manifests(), report("s2", "done"), 1000);
        for e in &doc.sessions {
            assert_eq!(e.stale_after, None);
            assert_eq!(effective_state(e, 10_000_000), e.state.as_str());
        }
    }

    #[test]
    fn seen_flips_done_to_idle_only() {
        let mut doc = Doc::empty();
        apply_report(&mut doc, &manifests(), report("s1", "done"), 1000);
        apply_report(&mut doc, &manifests(), report("s2", "blocked"), 1000);
        for e in doc.sessions.iter_mut() {
            if e.state == "done" {
                e.state = "idle".into();
            }
        }
        assert!(doc.sessions.iter().any(|e| e.state == "idle"));
        assert!(doc.sessions.iter().any(|e| e.state == "blocked"));
    }

    #[test]
    fn attention_order_is_written_into_the_file() {
        let mut doc = Doc::empty();
        apply_report(&mut doc, &manifests(), report("a-idle", "idle"), 1000);
        apply_report(&mut doc, &manifests(), report("b-block", "blocked"), 1000);
        apply_report(&mut doc, &manifests(), report("c-done", "done"), 1000);
        doc.sessions.sort_by_key(|e| (state_rank(&e.state), e.key.clone()));
        let states: Vec<&str> = doc.sessions.iter().map(|e| e.state.as_str()).collect();
        assert_eq!(states, vec!["blocked", "done", "idle"]);
    }

    #[test]
    fn hook_events_map_to_the_right_states() {
        let mk = |event: &str, extra: &str| -> HookPayload {
            serde_json::from_str(&format!(
                r#"{{"session_id":"s1","hook_event_name":"{event}","cwd":"/tmp"{extra}}}"#
            ))
            .unwrap()
        };
        let cases = [
            ("SessionStart", "", "idle"),
            ("UserPromptSubmit", r#","prompt":"fix the bug""#, "working"),
            ("PostToolUse", "", "working"),
            ("Notification", r#","message":"needs permission""#, "blocked"),
            ("Stop", "", "done"),
        ];
        for (event, extra, want) in cases {
            let r = hook_event_to_report(&mk(event, extra)).unwrap();
            assert_eq!(r.state, want, "{event}");
        }
        assert!(hook_event_to_report(&mk("SomethingNew", "")).is_none());
    }

    #[test]
    fn prompt_becomes_a_bounded_single_line_title() {
        let p: HookPayload = serde_json::from_str(
            r#"{"session_id":"s1","hook_event_name":"UserPromptSubmit","prompt":"line one\nline two\tend"}"#,
        )
        .unwrap();
        let r = hook_event_to_report(&p).unwrap();
        let t = r.title.unwrap();
        assert!(!t.contains('\n') && !t.contains('\t'));
        let long: String = "x".repeat(500);
        assert!(clean_line(&long, 120).chars().count() <= 121);
    }

    #[test]
    fn herdr_list_is_mirrored_without_self_reporting_kinds() {
        let raw = r#"{"id":"x","result":{"agents":[
            {"pane_id":"w1:p1","agent":"claude","agent_status":"working","cwd":"/a","name":"scout"},
            {"pane_id":"w1:p2","agent":"codex","agent_status":"blocked","cwd":"/b","name":"ox","terminal_title_stripped":"digging"},
            {"pane_id":"w1:p3","agent":"mystery","agent_status":"sideways","cwd":"/c"}
        ]}}"#;
        let mut doc = Doc::empty();
        apply_herdr_list(&mut doc, &manifests(), raw, 1000);
        // claude self-reports, so its pane is not mirrored
        assert!(doc.find_mut("herdr:w1:p1").is_none());
        let ox = doc.find_mut("herdr:w1:p2").unwrap();
        assert_eq!(ox.state, "blocked");
        assert_eq!(ox.agent, "codex");
        assert_eq!(ox.source, "herdr");
        assert_eq!(ox.stale_after, Some(1000 + HERDR_TTL_SECS));
        // an unknown kind still shows up, with its herdr name and an honest state
        let myst = doc.find_mut("herdr:w1:p3").unwrap();
        assert_eq!(myst.agent, "mystery");
        assert_eq!(myst.state, "unknown");
    }

    #[test]
    fn herdr_entries_vanish_when_herdr_stops_listing_them() {
        let raw = r#"{"result":{"agents":[{"pane_id":"w1:p2","agent":"codex","agent_status":"working"}]}}"#;
        let mut doc = Doc::empty();
        apply_herdr_list(&mut doc, &manifests(), raw, 1000);
        assert_eq!(doc.sessions.len(), 1);
        // server gone → empty list → mirror empties; self entries survive
        apply_report(&mut doc, &manifests(), report("s-self", "working"), 1000);
        apply_herdr_list(&mut doc, &manifests(), "", 1010);
        assert_eq!(doc.sessions.len(), 1);
        assert_eq!(doc.sessions[0].key, "s-self");
    }

    #[test]
    fn session_end_removes_and_prune_expires_the_pidless() {
        let mut doc = Doc::empty();
        apply_report(&mut doc, &manifests(), report("s1", "working"), 1000);
        assert!(remove_key(&mut doc, "s1"));
        apply_report(&mut doc, &manifests(), report("s2", "idle"), 1000);
        assert!(!prune(&mut doc, 1000 + NO_PID_EXPIRY_SECS - 1));
        assert!(prune(&mut doc, 1000 + NO_PID_EXPIRY_SECS + 1));
        assert!(doc.sessions.is_empty());
    }

    #[test]
    fn a_dead_pid_outlives_the_grace_window_and_no_longer() {
        let mut doc = Doc::empty();
        let mut r = report("s1", "working");
        r.pid = Some(4_000_000); // beyond pid_max on any default Linux
        apply_report(&mut doc, &manifests(), r, 1000);
        assert!(!prune(&mut doc, 1000 + DEAD_PID_GRACE_SECS - 5));
        assert_eq!(doc.sessions.len(), 1);
        assert!(prune(&mut doc, 1000 + DEAD_PID_GRACE_SECS + 5));
        assert!(doc.sessions.is_empty());
    }
}
