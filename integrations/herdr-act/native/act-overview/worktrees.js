// Worktree per Herdr terminal, shared with the status bar: a coordinator's terminal sits in the main checkout.
// Subagents can share their coordinator's terminal; the coordinator's own (aggregated) worktree wins.
export function worktreesByTerminal(rows) {
  const out = {};
  for (const { session } of rows) {
    const terminal = session.source?.Native?.terminal;
    if (!terminal || !session.worktree || (session.parent_id && out[terminal])) continue;
    out[terminal] = session.worktree;
  }
  for (const { session } of rows) {
    const terminal = session.source?.Native?.terminal;
    if (terminal && session.worktree && !session.parent_id) out[terminal] = session.worktree;
  }
  return out;
}
