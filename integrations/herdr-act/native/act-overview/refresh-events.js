// Coalesced native/file invalidations; one shared watcher pair serves the list and selected conversation.
import { watchFolder, shared, setShared } from "herdr";

export const FILE_EVENTS = "herdr-act.file-events";
const UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
const rollout = new RegExp("(?:^|/)[^/]*?(" + UUID + ")\\.jsonl$", "i");

export function nativeKey(spaceList, paneList, terminalList) {
  return JSON.stringify([
    spaceList.map(s => [s.id, s.name, s.path, s.tabs]),
    paneList.map(p => [p.pane, p.space, p.tab, p.terminal]),
    terminalList.map(t => [t.address, t.state, t.status, t.agent, t.cwd, t.name, t.parent, t.turn, t.report]),
  ]);
}

export function classifyChanges(kind, root, paths, known = []) {
  const result = { snapshot: false, catalog: false, all: false, threads: [] };
  const directory = root.replace(/\\/g, "/").replace(/\/$/, "");
  for (const changed of paths) {
    const path = changed.replace(/\\/g, "/").replace(/\/$/, "");
    if (path === directory) {
      result.snapshot = true;
      if (kind === "codex") { result.catalog = true; result.all = true; }
      continue;
    }
    if (!path.startsWith(directory + "/")) continue;
    const relative = path.slice(directory.length + 1);
    if (kind === "overview") {
      if (relative === "agent-overview.json" || /^agent-overview\/sessions\/[^/]+\.json$/.test(relative)
        || relative === "agent-overview/codex-cache/snapshot.json") result.snapshot = true;
    } else if (relative.startsWith("sessions/")) {
      const id = relative.match(rollout)?.[1]?.toLowerCase();
      if (id) {
        result.threads.push(id);
        if (!known.includes(id)) { result.catalog = true; result.snapshot = true; }
      }
    } else if (/^state_\d+\.sqlite(?:-wal)?$/.test(relative) || relative === "session_index.jsonl"
      || relative === "app-server-control/app-server-control.sock") {
      result.catalog = true; result.snapshot = true;
    }
  }
  result.threads = [...new Set(result.threads)];
  return result;
}

export function queueRefresh(host, cx) {
  if (host.refreshQueued) return;
  host.refreshQueued = true;
  cx.timer.after(180, cx => { host.refreshQueued = false; host.refresh(cx); });
}

export function sourceChanged(host, kind, root, paths, cx) {
  const change = classifyChanges(kind, root, paths, host.knownThreads);
  host.knownThreads = [...new Set([...(host.knownThreads ?? []), ...change.threads])];
  if (change.catalog) {
    host.catalogPending = true;
    host.catalogRevision = (host.catalogRevision ?? 0) + 1;
  }
  if (change.threads.length || change.all) {
    const previous = shared(FILE_EVENTS) ?? {}, revision = (previous.revision ?? 0) + 1;
    const threads = { ...previous.threads };
    for (const id of change.threads) threads[id] = revision;
    const recent = Object.entries(threads).sort((a, b) => b[1] - a[1]).slice(0, 512);
    setShared(FILE_EVENTS, { revision, all: change.all ? revision : previous.all, threads: Object.fromEntries(recent) });
  }
  if (change.snapshot) queueRefresh(host, cx);
}

export function watchSources(host, snapshot, cx) {
  const ids = snapshot.rows.map(r => r.session.id).filter(id => id.startsWith("codex-thread:")).map(id => id.slice(13));
  host.knownThreads = [...new Set([...(host.knownThreads ?? []), ...ids])];
  for (const [kind, root] of Object.entries(snapshot.watch ?? {})) {
    if (!root || host.watchPaths?.[kind] === root) continue;
    host.watchPaths = { ...host.watchPaths, [kind]: root };
    host.watchWarnings = { ...host.watchWarnings, [kind]: "" };
    cx.spawn(async cx => {
      try {
        while (host.watchPaths?.[kind] === root) {
          const paths = await watchFolder(root);
          if (host.watchPaths?.[kind] !== root) break;
          sourceChanged(host, kind, root, paths, cx);
        }
      } catch (error) {
        host.watchPaths = { ...host.watchPaths, [kind]: null };
        host.watchWarnings = { ...host.watchWarnings, [kind]: "Dateibeobachtung ausgefallen; Kontrollabfrage bleibt aktiv: " + String(error) };
        cx.notify();
      }
    });
    // Catch changes between the initial read and subscription without a retry loop on watcher failures.
    if (!host.watchCatchup?.includes(root)) {
      host.watchCatchup = [...(host.watchCatchup ?? []), root];
      queueRefresh(host, cx);
    }
  }
}

export function snapshotReceived(host, snapshot, cx, revision) {
  watchSources(host, snapshot, cx);
  const cache = snapshot.codex;
  if (cache?.started && (host.catalogRevision ?? 0) === revision) host.catalogPending = false;
  if (!host.catalogPending || !cache || cache.refreshing || host.catalogTimer) return;
  host.catalogTimer = true;
  // One delayed follow-up preserves a change received during the daemon cache's cooldown.
  cx.timer.after(Math.max(1000, cache.refreshAfterMs ?? 30_000), cx => {
    host.catalogTimer = false;
    if (host.catalogPending) queueRefresh(host, cx);
  });
}

export function conversationChanged(host, id, cx) {
  const events = shared(FILE_EVENTS) ?? {};
  const ids = host.conversation?.id === id ? host.conversation.data?.watchThreads : null;
  const version = Math.max(events.all ?? 0, ...(ids ?? [id.slice(13)]).map(key => events.threads?.[key] ?? 0));
  const signal = id + ":" + version;
  if (host.conversationSignal === signal) return;
  host.conversationSignal = signal;
  if (host.conversation?.id === id) host.readConversation(cx);
}

export function watchError(host) { return Object.values(host.watchWarnings ?? {}).filter(Boolean).join(" · "); }
