// Close only the confirmed live session, then persist its ended and hidden state.
// A subagent running inside its parent's terminal is closed in the overview only: stopping that terminal would stop the parent.
// A session with nothing left to stop is closed in the overview as well, so it leaves the list.
function insideParent(session, rows) {
  const terminal = session.source?.Native?.terminal;
  const parent = rows.find(row => row.session.id === session.parent_id)?.session;
  return Boolean(terminal && parent && parent.source?.Native?.terminal === terminal);
}
export async function closeSession(session, exec, binary) {
  const call = async (command, args) => {
    const result = await exec(command, args);
    if (result.code !== 0) throw new Error(result.stderr || "Die Sitzung konnte nicht geschlossen werden.");
    return result.stdout;
  };
  const snapshot = JSON.parse(await call(binary, ["agents", "native-snapshot"]));
  const current = snapshot.rows.find(row => row.session.id === session.id)?.session;
  if (!current) throw new Error("Die Sitzung ist nicht mehr verfügbar. Aktualisiere die Übersicht.");
  if ((current.activity_generation ?? null) !== (session.activity_generation ?? null)) {
    throw new Error("Die Sitzung hat inzwischen neue Arbeit. Wähle sie erneut aus.");
  }
  const terminal = current.source?.Native?.terminal, thread = current.source?.CodexApp?.thread_id;
  if (insideParent(current, snapshot.rows)) {
    // Nothing to stop: the parent owns the terminal.
  } else if (typeof terminal === "string" && terminal.trim()) {
    await call("hc", ["kill", "--", terminal]);
  } else if (typeof thread === "string" && /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(thread) && current.id === "codex-thread:" + thread) {
    await call("codex", ["archive", thread]);
  } else if (current.source?.Native || current.source?.CodexApp) {
    throw new Error("Für diese Sitzung ist kein laufender Agent erreichbar. Du kannst sie ausblenden.");
  }
  try {
    await call(binary, ["agents", "native-update", JSON.stringify({
      id: current.id, closed: true, hidden: true, generation: current.activity_generation
    })]);
  } catch (error) {
    throw new Error("Die Sitzung ist geschlossen. Die Anzeige konnte nicht gespeichert werden: " + error.message);
  }
}
// What closing does to this session, in the words of the confirmation.
export function closeEffect(session, rows) {
  if (insideParent(session, rows)) return "Der Unteragent verschwindet aus der Übersicht. Der Hauptagent läuft weiter.";
  if (session.source?.Native) return "Das Terminal und seine Terminal-Unteragenten werden beendet. Laufende Arbeit wird unterbrochen.";
  if (session.source?.CodexApp) return "Die Codex-Sitzung wird archiviert und verschwindet aus der Übersicht. Ihr Verlauf bleibt erhalten.";
  return "Die Sitzung verschwindet aus der Übersicht.";
}
