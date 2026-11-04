// Project families, individual status marks, and automatic disclosure of new human questions.
import { completion, families, filtered } from "./model.js";

const STATES = {
  question: { kind: "question", icon: "question", color: "ansi3", label: "Braucht dich", rank: 0 },
  problem: { kind: "problem", icon: "problem", color: "ansi1", label: "Problem", rank: 1 },
  working: { kind: "working", icon: "working", color: "ansi4", label: "Arbeitet", rank: 2 },
  done: { kind: "done", icon: "done", color: "ansi2", label: "Auftrag abgeschlossen", rank: 3 },
  parent: { kind: "parent", icon: "waiting", color: "dim", label: "Wartet auf Hauptagent", rank: 4 },
  idle: { kind: "idle", icon: "idle", color: "dim", label: "Inaktiv", rank: 4 },
  unknown: { kind: "unknown", icon: "unknown", color: "dim", label: "Status unbekannt", rank: 5 }
};
export function statusOf(session) {
  if (session.question || session.needs_user || session.runtime_status === "blocked") return STATES.question;
  if (session.problem || ["error", "failed", "systemError"].includes(session.runtime_status)) return STATES.problem;
  if (completion(session) === true) return STATES.done;
  if (["working", "active"].includes(session.runtime_status)) return STATES.working;
  if (session.runtime_status === "waiting-parent") return STATES.parent;
  if (["idle", "done", "closed", "interrupted"].includes(session.runtime_status)) return STATES.idle;
  return STATES.unknown;
}
// Missing ACT data reads "Unbekannt": never blank, never "outside ACT".
export function phaseText(phase) {
  return ({ "Spec erstellen": "Spec", "Create Issues": "Issues", "Land Feature": "Land", "Außerhalb ACT": "Freie Arbeit" })[phase] ?? phase ?? "Unbekannt";
}
// Subagents show the ACT command they are running.
export function commandText(session) {
  const command = session.phase_history?.at(-1)?.command ?? "";
  return command.replace(/^act-/, "") || "Unbekannt";
}
function stored(row) {
  const s = row.session;
  return Boolean(s.source?.CodexApp) && ["notLoaded", "unknown", ""].includes(s.runtime_status ?? "")
    && !s.question && !s.needs_user && !s.problem;
}
const finished = family => family[0].group === "Done" || statusOf(family[0].session).kind === "done";
function urgency(families) {
  let status = STATES.unknown;
  for (const row of families.flat()) {
    const own = statusOf(row.session);
    if (own.rank < status.rank) status = own;
  }
  return status;
}
// Completed assignments stay in their own project with their check mark, after its open families.
// A project's mark is its most urgent open status; only a project with nothing open shows the check.
export function projectTree(rows, { project = "", showHidden = false } = {}) {
  const projects = new Map(), saved = [];
  for (const family of families(filtered(rows, "", showHidden))) {
    if (project && !family.some(row => row.session.project === project)) continue;
    if (!finished(family) && family.every(stored)) { saved.push(...family); continue; }
    const name = family[0].session.project || "Unknown";
    if (!projects.has(name)) projects.set(name, { name, open: [], closed: [], path: "" });
    const group = projects.get(name);
    (finished(family) ? group.closed : group.open).push(family);
    if (!group.path) group.path = family.find(row => row.session.project_path)?.session.project_path ?? "";
  }
  const list = [...projects.values()].map(({ name, open, closed, path }) => ({
    name, path, rows: [...open, ...closed].flat(),
    count: [...open, ...closed].length, open: open.length,
    status: open.length ? urgency(open) : STATES.done
  }));
  return { projects: list.sort((a, b) => a.status.rank - b.status.rank || a.name.localeCompare(b.name)), saved };
}
export function newQuestions(rows, previous) {
  const byId = new Map(rows.map(row => [row.session.id, row.session]));
  const keys = new Set(), agents = new Set(), projects = new Set();
  for (const { session } of rows) {
    if (statusOf(session).kind !== "question") continue;
    const key = `${session.id}:${session.activity_generation ?? ""}:${session.question ?? ""}`;
    keys.add(key);
    if (previous.has(key)) continue;
    const visited = new Set();
    for (let s = session; s && !visited.has(s.id); s = byId.get(s.parent_id)) {
      visited.add(s.id); agents.add(s.id); projects.add(s.project);
    }
  }
  return { keys, agents, projects };
}
