// Personal ACT layout: native agent list, real terminal selection, durable corrections, and workflow history.
import { View, div } from "gpui-kit";
import { v_flex, h_flex, InputState, Input } from "gpui-base";
import { theme, props, shared, setShared, focused, spaces, panes, terminals, run, exec, openOverlay, closeOverlay, setAgentOrder } from "herdr";
import { runtime, selected, completion, nextCompletion, sourceName, lightSnapshot, liveSnapshot, spaceSnapshot } from "./model.js";
import { newQuestions } from "./tree-model.js";
import { renderProjectTree } from "./tree-view.js";
import { closeSession } from "./lifecycle.js";
const BINARY = "/Users/oliver/.herdr-projects/data/agent-overview/bin/1cb43df4b3a38b486d4ad809ec359a54059187ed/herdr-projects";
const SNAPSHOT = "herdr-act.snapshot", SELECTION = "herdr-act.selected", PROJECT = "herdr-act.project", ALL_SPACES = "herdr-act.all-spaces", REFRESH = "herdr-act.refresh";
function button(id, title, action, t) {
  return div().id(id).px(9).py(5).rounded(6).text_size(11.5).text_color(t.muted).cursor_pointer().role("button").accessibility_label(title).tab_index(0)
    .on_key_down((event, cx) => { if (["enter", "space"].includes(event.keystroke)) { cx.stop_propagation(); action(event, cx); } })
    .hover(s => s.bg(t.hover).text_color(t.foreground)).on_click(action).child(title);
}
export default class ActOverview extends View {
  init(_props, cx) {
    this.mode = props()?.mode ?? "list";
    this.tab = focused()?.tab;
    this.space = focused()?.space;
    this.snapshot = { rows: [], errors: [] };
    this.folded = new Set(JSON.parse(localStorage.getItem("project-tree.sections") ?? '["Done","Saved"]'));
    this.projectFolded = new Set(JSON.parse(localStorage.getItem("project-tree.projects") ?? "[]"));
    this.historyExpanded = new Set();
    this.seenQuestions = new Set();
    this.pages = {};
    this.childrenFolded = new Set();
    this.history = false;
    this.busy = false;
    this.opening = false;
    this.error = "";
    if (this.mode === "edit") {
      const s = props().session;
      this.fields = {};
      for (const [key, value] of Object.entries({ title: s.title, project: s.project, ticket: s.ticket, phase: s.act_phase })) {
        this.fields[key] = InputState.new({ value: value ?? "" });
        this.fields[key].on("change", (_e, cx) => cx.notify());
      }
    } else if (this.mode === "list") {
      this.refresh(cx);
      cx.timer.every(5000, cx => this.refresh(cx));
    }
  }
  refresh(cx) {
    if (this.busy) return;
    this.busy = true;
    cx.spawn(async cx => {
      try {
        const result = await exec(BINARY, ["agents", "native-snapshot"]);
        if (result.code !== 0) throw new Error(result.stderr || "Datenquelle nicht erreichbar");
        this.snapshot = liveSnapshot(JSON.parse(result.stdout), terminals());
        const attention = newQuestions(this.snapshot.rows, this.seenQuestions);
        for (const project of attention.projects) this.projectFolded.delete(project);
        for (const id of attention.agents) this.childrenFolded.delete(id);
        this.seenQuestions = attention.keys;
        setShared(SNAPSHOT, lightSnapshot(this.snapshot, shared(SELECTION), focused()?.terminal));
        this.error = "";
      } catch (e) { this.error = String(e); }
      this.busy = false; cx.notify();
    });
  }
  mutate(payload, cx, close = false) {
    cx.spawn(async cx => {
      try {
        const result = await exec(BINARY, ["agents", "native-update", JSON.stringify(payload)]);
        if (result.code !== 0) throw new Error(result.stderr);
        const fresh = await exec(BINARY, ["agents", "native-snapshot"]);
        if (fresh.code !== 0) throw new Error(fresh.stderr);
        setShared(SNAPSHOT, lightSnapshot(JSON.parse(fresh.stdout), shared(SELECTION), focused()?.terminal));
        setShared(REFRESH, (shared(REFRESH) ?? 0) + 1);
        if (close) closeOverlay(props().overlay);
        this.error = "";
      } catch (e) { this.error = String(e); }
      cx.notify();
    });
  }
  choose(row, cx) {
    const s = row.session;
    if (s.source?.Native && !terminals().some(t => t.address === s.source.Native.terminal && t.state === "live")) {
      this.refresh(cx);
      return;
    }
    setShared(SELECTION, s.id);
    setShared(SNAPSHOT, lightSnapshot(this.snapshot, s.id, s.source?.Native?.terminal ?? focused()?.terminal));
    if (s.source?.Native) {
      run("layout.show", { id: "act-terminal" });
      run("layout.resize", { id: "act-context", px: 190 });
      const pane = panes().find(p => p.terminal === s.source.Native.terminal);
      if (pane) run("pane.focus", { pane: pane.pane });
      else run("terminal.peek", { terminal: s.source.Native.terminal });
    } else {
      run("layout.hide", { id: "act-terminal" });
      run("layout.resize", { id: "act-context", weight: 1 });
    }
  }
  openInHerdr(s, cx) {
    if (this.opening) return;
    this.opening = true;
    cx.spawn(async cx => {
      try {
        const snapshot = liveSnapshot(shared(SNAPSHOT) ?? { rows: [] }, terminals());
        let terminal = snapshot.rows.find(row => row.session.id === s.id && row.session.source?.Native)?.session.source.Native.terminal;
        if (!terminal) {
          const result = await exec("herdr-app", ["do", "terminal.open", JSON.stringify({
            argv: ["codex", "resume", "--no-daemon", s.source.CodexApp.thread_id], folder: s.project_path, name: s.title
          })]);
          if (result.code !== 0) throw new Error(result.stderr || "Die Unterhaltung konnte nicht geöffnet werden.");
          terminal = JSON.parse(result.stdout).terminal;
          if (!terminal) throw new Error("Herdr hat kein Terminal geöffnet.");
        }
        setShared(SELECTION, null);
        run("layout.show", { id: "act-terminal" });
        run("layout.resize", { id: "act-context", px: 190 });
        run("agent.focus", { terminal });
        this.error = "";
      } catch (e) { this.error = e.message ?? String(e); }
      this.opening = false; cx.notify();
    });
  }
  openInDesktop(s, cx) {
    cx.spawn(async cx => {
      try {
        const result = await exec("/usr/bin/open", ["codex://threads/" + encodeURIComponent(s.source.CodexApp.thread_id)]);
        if (result.code !== 0) throw new Error(result.stderr || "Codex Desktop konnte nicht geöffnet werden.");
        this.error = "";
      } catch (e) { this.error = e.message ?? String(e); }
      cx.notify();
    });
  }
  closeSession(cx) {
    if (this.closing) return;
    this.closing = true;
    this.error = "";
    cx.notify();
    const session = props().session, overlay = props().overlay;
    cx.spawn(async cx => {
      try {
        await closeSession(session, exec, BINARY);
        setShared(SELECTION, null);
        setShared("herdr-act.hidden", false);
        setShared(REFRESH, (shared(REFRESH) ?? 0) + 1);
        run("layout.show", { id: "act-terminal" });
        run("layout.resize", { id: "act-context", px: 190 });
        closeOverlay(overlay);
      } catch (e) { this.error = e.message ?? String(e); }
      this.closing = false; cx.notify();
    });
  }
  closeDialog(cx) {
    const t = theme(), s = props().session;
    return v_flex().w(460).p(20).gap(12).bg(t.popover).text_color(t.foreground)
      .child(div().font_semibold().child(s.parent_id ? "Unteragent schließen?" : "Agent schließen?"))
      .child(div().text_size(13).child(s.title))
      .child(div().text_size(12).text_color(t.muted).child(s.source?.Native
        ? "Das Terminal und seine Terminal-Unteragenten werden beendet. Laufende Arbeit wird unterbrochen."
        : "Die Codex-Sitzung wird geschlossen und archiviert. Ihr Verlauf bleibt erhalten."))
      .when(Boolean(this.error), el => el.child(div().text_size(12).text_color(t.ansi1).child(this.error)))
      .child(h_flex().justify_end().gap(8)
        .child(button("cancel", "Abbrechen", () => closeOverlay(props().overlay), t).disabled(Boolean(this.closing)))
        .child(button("confirm-close", this.closing ? "Wird geschlossen …" : "Schließen", (_e, cx) => this.closeSession(cx), t)
          .text_color(t.ansi1).disabled(Boolean(this.closing))));
  }
  fold(group, cx) {
    if (!this.folded.delete(group)) this.folded.add(group);
    localStorage.setItem("project-tree.sections", JSON.stringify([...this.folded])); cx.notify();
  }
  foldProject(project, cx) {
    if (!this.projectFolded.delete(project)) this.projectFolded.add(project);
    localStorage.setItem("project-tree.projects", JSON.stringify([...this.projectFolded])); cx.notify();
  }
  detail(rows, cx) {
    const t = theme(), row = selected(rows, shared(SELECTION), focused()?.terminal);
    if (!row) return div().p(18).text_color(t.muted).child("Wähle links eine Agentensitzung.");
    const s = row.session, complete = completion(s);
    return v_flex().id("act-details").size_full().min_h(0).overflow_y_scrollbar().p(14).gap(7).bg(t.surface).text_color(t.foreground).text_size(12)
      .child(h_flex().gap(8).items_center().child(div().min_w(0).flex_1().font_semibold().truncate().child(s.title))
        .child(button("edit", "Korrigieren", () => openOverlay({ mod: "act-overview", kind: "dialog", width: 500, props: { mode: "edit", session: s } }), t))
        .when(Boolean(s.source?.Native || s.source?.CodexApp), el => el.child(button("close", "Schließen …", () => openOverlay({
          mod: "act-overview", kind: "dialog", width: 460, props: { mode: "close", session: s }
        }), t))))
      .child(div().text_color(t.muted).child(`${s.project} · ${s.program} · ${sourceName(s)}`))
      .child(div().child(`${runtime(s.runtime_status)} · Auftrag: ${complete === true ? "Fertig" : complete === false ? "Nicht abgeschlossen" : "Unbekannt"}`))
      .child(div().child(`ACT: ${s.act_phase ?? "Unbekannt"} · ${s.phase_source ?? "Keine Phasenmeldung"}`))
      .child(div().text_color(t.muted).child(`Spec/Ticket: ${s.ticket ?? "Unbekannt"}`))
      .when(Boolean(s.source?.CodexApp), el => el.child(v_flex().items_start().gap(4)
        .child(button("open-herdr", this.opening ? "Wird geöffnet …" : "In Herdr öffnen", (_e, cx) => this.openInHerdr(s, cx), t))
        .child(button("open-desktop", "In Codex Desktop öffnen", (_e, cx) => this.openInDesktop(s, cx), t))))
      .when(Boolean(this.error), el => el.child(div().text_color(t.ansi1).child(this.error)))
      .when(Boolean(s.question), el => el.child(div().text_color(t.ansi3).child(s.question)))
      .when(Boolean(s.problem), el => el.child(div().text_color(t.ansi1).child(s.problem)))
      .child(h_flex().gap(6)
        .child(button("workflow", `${this.history ? "▾" : "▸"} Verlauf (${s.phase_history.length})`, (_e, cx) => { this.history = !this.history; cx.notify(); }, t))
        .child(button("complete", s.manual_completion == null ? "Als fertig markieren" : s.manual_completion ? "Als offen markieren" : "Automatisch erkennen", (_e, cx) => this.mutate({ id: s.id, completion: nextCompletion(s), generation: s.activity_generation }, cx), t))
        .child(button("hide", s.hidden ? "Einblenden" : "Ausblenden", (_e, cx) => this.mutate({ id: s.id, hidden: !s.hidden, generation: s.activity_generation }, cx), t)))
      .when(this.history, el => el.children([...s.phase_history].reverse().map((phase, i) => div().id("phase-" + i).text_color(t.muted).child(`${phase.phase} · ${phase.command ?? "manuell"} · ${phase.source}${phase.ended_at ? " · beendet" : ""}`))));
  }
  editor(cx) {
    const t = theme(), s = props().session;
    return v_flex().w(500).p(20).gap(10).bg(t.popover).text_color(t.foreground)
      .child(div().font_semibold().child("Zuordnung korrigieren"))
      .children(Object.entries({ title: "Auftrag", project: "Projekt", ticket: "Spec/Ticket", phase: "ACT-Schritt" }).map(([key, label]) => v_flex().gap(5)
        .child(div().text_size(12).text_color(t.muted).child(label))
        .child(Input.new(this.fields[key]).h(34).px(8).rounded(6).border_1().border_color(t.input).bg(t.background).text_color(t.foreground))))
      .when(Boolean(this.error), el => el.child(div().text_color(t.ansi1).child(this.error)))
      .child(h_flex().justify_end().gap(8).child(button("cancel", "Abbrechen", () => closeOverlay(props().overlay), t))
        .child(button("save", "Speichern", (_e, cx) => this.mutate({ id: s.id, fields: Object.fromEntries(Object.entries(this.fields).map(([key, input]) => [key, input.value()])) }, cx, true), t)));
  }
  render(cx) {
    if (this.mode === "edit") return this.editor(cx);
    if (this.mode === "close") return this.closeDialog(cx);
    if (this.mode === "list" && this.refreshRequest !== shared(REFRESH)) {
      this.refreshRequest = shared(REFRESH); this.refresh(cx);
    }
    const focus = focused(), all = shared(ALL_SPACES) === true;
    if (this.mode === "list" && this.allSpaces !== all) { this.allSpaces = all; this.pages = {}; }
    if (this.mode === "list" && (this.tab !== focus?.tab || this.space !== focus?.space)) {
      if (this.space !== focus?.space && !all) { setShared(PROJECT, ""); this.pages = {}; }
      this.space = focus?.space;
      this.tab = focus?.tab; setShared(SELECTION, null);
      run("layout.show", { id: "act-terminal" });
      run("layout.resize", { id: "act-context", px: 190 });
    }
    const space = spaces().find(item => item.id === focus?.space), t = theme();
    const full = liveSnapshot(this.mode === "list" ? this.snapshot : (shared(SNAPSHOT) ?? { rows: [], errors: [] }), terminals());
    const snapshot = spaceSnapshot(full, space, panes(), all);
    if (this.mode === "detail") return this.detail(snapshot.rows, cx);
    const project = shared(PROJECT) ?? "", hidden = shared("herdr-act.hidden") === true;
    if (this.mode === "projects") return v_flex().w(300).max_h(500).overflow_y_scrollbar().p(8).bg(t.popover)
      .children(["", ...(snapshot.projects ?? [])].map(p => button("project-" + p, p || "Alle Projekte", () => { setShared(PROJECT, p); closeOverlay(props().overlay); }, t)));
    const current = selected(snapshot.rows, shared(SELECTION), focused()?.terminal);
    const list = renderProjectTree(this, snapshot, project, hidden, current, t);
    setAgentOrder(this.shownRows.filter(r => r.session.source?.Native).map(r => r.session.source.Native.terminal));
    return v_flex().id("act-root").size_full().min_w(0).min_h(0).bg(t.chrome).text_color(t.foreground).text_size(13)
      .child(h_flex().px(14).pt(16).pb(8).items_center().child(div().flex_1().font_semibold().text_size(15).child("ACT · Agenten"))
        .child(button("refresh", "↻", (_e, cx) => this.refresh(cx), t)))
      .child(h_flex().px(8).gap(4).child(button("projects", `Projekt: ${project || "Alle"} ▾`, () => openOverlay({ mod: "act-overview", kind: "popup", width: 300, height: 450, props: { mode: "projects" } }), t))
        .child(button("hidden", hidden ? "Alle anzeigen ✓" : "Ausgeblendete", () => setShared("herdr-act.hidden", !hidden), t)))
      .when(snapshot.rows.length > 0, el => el.child(list))
      .when(!snapshot.rows.length, el => el.child(div().p(14).text_color(t.muted).child(this.busy ? "Agentendaten werden geladen …" : all ? "Keine Agentensitzungen gefunden." : "Keine Agenten mit diesem Space verbunden. Gespeicherte Gespräche findest du unter „All Spaces“.")))
      .when(Boolean(snapshot.codex?.refreshing), el => el.child(div().p(10).text_size(11).text_color(t.muted).child("Codex-Daten werden aktualisiert …")))
      .when(Boolean(this.error || snapshot.errors.length), el => el.child(div().p(10).text_size(11).text_color(t.ansi1).child(this.error || snapshot.errors.join(" · "))));
  }
}
