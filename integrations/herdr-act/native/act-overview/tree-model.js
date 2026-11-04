// Project families, individual status marks, and automatic disclosure of new human questions.
import { completion, families, filtered } from "./model.js";

const STATES = {
  question: { kind: "question", icon: "question", color: "ansi3", label: "Braucht dich", rank: 0 },
  problem: { kind: "problem", icon: "problem", color: "ansi1", label: "Problem", rank: 1 },
  working: { kind: "working", icon: "working", color: "ansi4", label: "Arbeitet", rank: 2 },
  done: { kind: "done", icon: "done", color: "ansi2", label: "Auftrag abgeschlossen", rank: 3 },
  parent: { kind: "parent", icon: "parent", color: "dim", label: "Wartet auf Hauptagent", rank: 4 },
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
export function phaseText(phase) {
  return ({ "Spec erstellen": "Spec", "Create Issues": "Issues", "Land Feature": "Land", "Außerhalb ACT": "Freie Arbeit" })[phase] ?? phase ?? "";
}
// Subagents show the ACT command they are running; no reported command stays blank.
export function commandText(session) {
  const command = session.phase_history?.at(-1)?.command ?? "";
  return command.replace(/^act-/, "");
}
function stored(row) {
  const s = row.session;
  return Boolean(s.source?.CodexApp) && ["notLoaded", "unknown", ""].includes(s.runtime_status ?? "")
    && !s.question && !s.needs_user && !s.problem;
}
export function projectTree(rows, { project = "", showHidden = false } = {}) {
  const projects = new Map(), done = [], saved = [];
  for (const family of families(filtered(rows, "", showHidden))) {
    if (project && !family.some(row => row.session.project === project)) continue;
    if (family[0].group === "Done") { done.push(...family); continue; }
    if (family.every(stored)) { saved.push(...family); continue; }
    const name = family[0].session.project || "Unknown";
    if (!projects.has(name)) projects.set(name, { name, rows: [], count: 0, status: STATES.unknown });
    const group = projects.get(name);
    group.rows.push(...family);
    group.count += family.filter(row => row.depth === 0).length;
    for (const row of family) {
      const status = statusOf(row.session);
      if (status.rank < group.status.rank) group.status = status;
    }
  }
  return { projects: [...projects.values()].sort((a, b) => a.status.rank - b.status.rank || a.name.localeCompare(b.name)), done, saved };
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
