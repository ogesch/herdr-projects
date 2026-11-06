// Herdr notifications for new ACT questions; clicking one focuses the agent's terminal.
// Every list copy shares one record of announced questions, so no question is announced twice,
// and the first look after a start only records what is already open.
import { questionKey } from "./tree-model.js";

export const ANNOUNCED = "herdr-act.announced-questions";

// Pure: which fresh questions to announce. Skipped: the terminal you are in, a terminal Herdr already
// reports as blocked (Herdr posts its own note then), and sessions without a Herdr terminal to go to.
export function questionAlerts(fresh, announced, liveTerminals, focusedTerminal) {
  if (!Array.isArray(announced)) return [];
  const seen = new Set(announced);
  const status = new Map(liveTerminals.map(t => [t.address, t.status]));
  const alerts = [];
  for (const s of fresh) {
    const terminal = s.source?.Native?.terminal;
    if (seen.has(questionKey(s)) || !terminal || !status.has(terminal)) continue;
    if (terminal === focusedTerminal || status.get(terminal) === "blocked") continue;
    alerts.push({
      title: `${s.project || "Agent"} braucht dich`,
      body: (s.question || s.title || "Offene Frage").split("\n")[0],
      level: "warning",
      action: { name: "agent.focus", args: { terminal } }
    });
  }
  return alerts;
}

export function announceQuestions(attention, { shared, setShared, notify, terminals, focused }) {
  const announced = shared(ANNOUNCED);
  for (const alert of questionAlerts(attention.fresh ?? [], announced, terminals(), focused()?.terminal)) notify(alert);
  const keys = [...attention.keys];
  if (!Array.isArray(announced) || keys.length !== announced.length || keys.some(k => !announced.includes(k))) setShared(ANNOUNCED, keys);
}
