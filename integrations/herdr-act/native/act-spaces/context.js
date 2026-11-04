// Resolve a Space's live folder and display its Git identity independently of its free name.
export function place(space, panes) {
  const tabs = new Set((space.tabs ?? []).map(tab => tab.id));
  return panes.find(pane => tabs.has(pane.tab) && pane.cwd)?.cwd || space.folder;
}
export const where = (space, panes) => JSON.stringify([space.machine ?? null, place(space, panes)]);
export const short = path => (path ?? "").replace(/^\/Users\/[^/]+/, "~");
export const base = path => path.replace(/\/+$/, "").split("/").pop();
export function describeSpace({ w, g, cwd, known }) {
  return {
    repository: g ? `Repo: ${g.name} · ${g.branch || "detached"}` : known ? "Kein Git-Repo" : "Repo: unbekannt",
    folder: cwd || w.folder || "Arbeitsordner unbekannt",
    // The row's one quiet line: branch, then the folder shortened to ~.
    line: [g ? g.branch || "losgelöst" : known ? "kein Git" : null, short(cwd || w.folder) || "Arbeitsordner unbekannt"].filter(Boolean).join(" · ")
  };
}
