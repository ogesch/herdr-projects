// Close only the confirmed live session, then persist its ended and hidden state.
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
  if (typeof terminal === "string" && terminal.trim()) {
    await call("hc", ["kill", "--", terminal]);
  } else if (typeof thread === "string" && /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(thread) && current.id === "codex-thread:" + thread) {
    await call("codex", ["archive", thread]);
  } else {
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
