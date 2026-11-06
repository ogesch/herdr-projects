// Project families, individual status marks, and automatic disclosure of new human questions.
import { completion, families, filtered } from "./model.js";

const STATES = {
  question: { kind: "question", icon: "question", color: "ansi3", label: "Braucht dich", rank: 0 },
  problem: { kind: "problem", icon: "problem", color: "ansi1", label: "Problem", rank: 1 },
  working: { kind: "working", icon: "working", color: "working", label: "Arbeitet", rank: 2 },
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
// Within a project: stopped agents first, the one that stopped last on top; working agents below in
// their own order (their time changes constantly, so sorting them would make the list jump).
const working = family => statusOf(family[0].session).kind === "working";
const stoppedAt = family => family[0].session.activity_generation ?? 0;
export function recentFirst(open) {
  const stopped = open.filter(f => !working(f)).sort((a, b) => stoppedAt(b) - stoppedAt(a));
  return [...stopped, ...open.filter(working)];
}
// Derive connectors from visible siblings so folded branches leave no dangling lines.
export function branchGuides(rows) {
  const byId = new Map(rows.map(row => [row.session.id, row.session]));
  const parents = new Set(), following = new Set(), guides = new Map();
  for (const { session } of [...rows].reverse()) {
    if (parents.has(session.parent_id)) following.add(session.id);
    parents.add(session.parent_id);
  }
  for (const { session } of rows) {
    const columns = [], visited = new Set([session.id]);
    for (let node = session; byId.has(node.parent_id) && !visited.has(node.parent_id); node = byId.get(node.parent_id)) {
      visited.add(node.parent_id);
      columns.unshift(node === session ? (following.has(node.id) ? "branch" : "last") : (following.has(node.id) ? "stem" : "blank"));
    }
    guides.set(session.id, columns);
  }
  return guides;
}
function urgency(families) {
  let status = STATES.unknown;
  for (const row of families.flat()) {
    const own = statusOf(row.session);
    if (own.rank < status.rank) status = own;
  }
  return status;
}
// Completed assignments stay in their own project with their check mark, after its open families,
// which are ordered by recentFirst.
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
    name, path, rows: [...recentFirst(open), ...closed].flat(),
    count: [...open, ...closed].length, open: open.length,
    status: open.length ? urgency(open) : STATES.done
  }));
  return { projects: list.sort((a, b) => a.status.rank - b.status.rank || a.name.localeCompare(b.name)), saved };
}
export const questionKey = s => `${s.id}:${s.activity_generation ?? ""}:${s.question ?? ""}`;
export function newQuestions(rows, previous) {
  const byId = new Map(rows.map(row => [row.session.id, row.session]));
  const keys = new Set(), agents = new Set(), projects = new Set(), fresh = [];
  for (const { session } of rows) {
    if (statusOf(session).kind !== "question") continue;
    const key = questionKey(session);
    keys.add(key);
    if (previous.has(key)) continue;
    fresh.push(session);
    const visited = new Set();
    for (let s = session; s && !visited.has(s.id); s = byId.get(s.parent_id)) {
      visited.add(s.id); agents.add(s.id); projects.add(s.project);
    }
  }
  return { keys, agents, projects, fresh };
}
