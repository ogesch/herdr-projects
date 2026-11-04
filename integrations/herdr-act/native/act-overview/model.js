// Native overview filtering preserves family attention, hidden assignments, and independent runtime states.
export function runtime(value) {
  return ({ working: "Arbeitet", active: "Arbeitet", blocked: "Wartet auf dich", waiting: "Wartet auf Eingabe", "waiting-parent": "Wartet auf Hauptagent", idle: "Inaktiv", done: "Inaktiv", closed: "Beendet", error: "Fehler", notLoaded: "Unbekannt", unknown: "Unbekannt" })[value] ?? value ?? "Unbekannt";
}
export function filtered(rows, project, hidden, collapsed = new Set()) {
  const byId = new Map(rows.map(r => [r.session.id, r]));
  return rows.filter(row => {
    if (project && row.session.project !== project) return false;
    if (hidden) return true;
    let current = row;
    for (let i = 0; current && i <= rows.length; i++) {
      if (current.session.hidden) return false;
      if (current !== row && collapsed.has(current.session.id)) return false;
      current = byId.get(current.session.parent_id);
    }
    return true;
  });
}
export function embeddedChild(session, rows) {
  const address = session.source?.Native?.terminal;
  const parent = rows.find(row => row.session.id === session.parent_id)?.session;
  return Boolean(address && session.id.startsWith("codex-thread:") && parent?.source?.Native?.terminal === address);
}
export function ownerRow(session, rows) {
  const visited = new Set();
  let row = rows.find(row => row.session.id === session.parent_id);
  while (row && embeddedChild(row.session, rows) && !visited.has(row.session.id)) {
    visited.add(row.session.id);
    row = rows.find(next => next.session.id === row.session.parent_id);
  }
  return row && !embeddedChild(row.session, rows) ? row : null;
}
export function selected(rows, id, terminal) {
  const explicit = rows.find(r => r.session.id === id);
  if (explicit && (embeddedChild(explicit.session, rows) || !explicit.session.source?.Native || explicit.session.source.Native.terminal === terminal)) return explicit;
  return rows.find(r => terminal && r.session.source?.Native?.terminal === terminal);
}
function subsetSnapshot(snapshot, visible) {
  if (snapshot.rows.every(visible)) return snapshot;
  const rows = snapshot.rows.filter(visible), ids = new Set(rows.map(r => r.session.id));
  const byId = new Map(snapshot.rows.map(r => [r.session.id, r]));
  return { ...snapshot, rows: rows.map(row => {
    let parent = row.session.parent_id, depth = row.depth, hidden = row.session.hidden;
    const visited = new Set([row.session.id]);
    // Keep visibility and indentation inherited from ancestors removed from the live view.
    for (let id = parent; id && !visited.has(id);) {
      visited.add(id);
      const ancestor = byId.get(id);
      if (!ancestor) break;
      if (!ids.has(id)) {
        if (parent === id) parent = ancestor.session.parent_id ?? null;
        depth = Math.max(0, depth - 1);
        if (ancestor.session.hidden) hidden = true;
      }
      id = ancestor.session.parent_id;
    }
    return parent === row.session.parent_id && depth === row.depth && hidden === row.session.hidden
      ? row : { ...row, depth, session: { ...row.session, parent_id: parent, hidden } };
  }) };
}
export function liveSnapshot(snapshot, terminals) {
  const live = new Set(terminals.filter(t => t.state === "live").map(t => t.address));
  return subsetSnapshot(snapshot, row => !row.session.source?.Native || live.has(row.session.source.Native.terminal));
}
export function spaceSnapshot(snapshot, space, panes, all = false) {
  if (all) return snapshot;
  const tabs = new Set((space?.tabs ?? []).map(tab => tab.id));
  const terminalTabs = new Map(panes.filter(pane => pane.terminal).map(pane => [pane.terminal, pane.tab]));
  const byId = new Map(snapshot.rows.map(row => [row.session.id, row.session]));
  const scoped = subsetSnapshot(snapshot, row => {
    const visited = new Set();
    for (let session = row.session; session && !visited.has(session.id); session = byId.get(session.parent_id)) {
      visited.add(session.id);
      const tab = terminalTabs.get(session.source?.Native?.terminal);
      if (tab !== undefined) return tabs.has(tab);
    }
    return false;
  });
  return { ...scoped, projects: [...new Set(scoped.rows.map(row => row.session.project).filter(Boolean))].sort() };
}
export const completion = s => s.manual_completion ?? s.task_complete;
export const nextCompletion = s => s.manual_completion == null ? true : s.manual_completion ? false : null;
const PROGRAMS = { claude: "Claude", codex: "Codex", gemini: "Gemini", opencode: "OpenCode", cursor: "Cursor", amp: "Amp", pi: "Pi" };
// Program identity for the list's icon and the detail line; Herdr ships these icons in ~/.config/herdr-app/icons.
export function programOf(s) {
  if (s.source?.CodexApp) return { icon: "icons/codex.svg", name: "Codex App" };
  const key = String(s.program ?? "").replace(/-code$/, "");
  return { icon: PROGRAMS[key] ? `icons/${key}.svg` : "icons/agent.svg", name: PROGRAMS[key] ?? s.program ?? "Unbekannt" };
}
export const sourceName = s => s.source?.Native ? "Herdr-App" : s.source?.CodexApp ? "Codex-App" : "Beendete Sitzung";
export function lightSnapshot(snapshot, id, terminal) {
  const current = selected(snapshot.rows, id, terminal);
  return {
    rows: snapshot.rows.filter(r => r.session.source?.Native || r.session.id === current?.session.id),
    projects: [...new Set(snapshot.rows.map(r => r.session.project).filter(Boolean))].sort(),
    errors: snapshot.errors ?? []
  };
}
export function families(rows) {
  const result = [];
  for (const row of rows) {
    if (!row.depth || !result.length) result.push([]);
    result[result.length - 1].push(row);
  }
  return result;
}
export function pageFamilies(rows, page = 0, size = 12) {
  const blocks = families(rows);
  const live = blocks.filter(f => f[0].session.source?.Native);
  const other = blocks.filter(f => !f[0].session.source?.Native);
  const pages = Math.max(1, Math.ceil(other.length / size));
  page = Math.min(Math.max(0, page), pages - 1);
  return { rows: [...live.flat(), ...other.slice(page * size, (page + 1) * size).flat()], page, pages };
}
