// Compact project headers, individual status icons, nested agents, and folded saved conversations.
import { div, svg } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { pageFamilies } from "./model.js";
import { projectTree, statusOf, phaseText, commandText } from "./tree-model.js";

const ICONS = "mods/act-overview/icons/";
function mark(status, t, id) {
  return h_flex().id("status-" + id).w(16).h(20).flex_none().items_center().justify_center()
    .tooltip(status.label).accessibility_label(status.label)
    .child(svg(ICONS + status.icon + ".svg").size(13).text_color(t[status.color] ?? t.dim));
}
function disclosure(id, open, action, t) {
  return h_flex().id("disclose-" + id).w(20).h(26).flex_none().items_center().justify_center().cursor_pointer()
    .tooltip(open ? "Einklappen" : "Aufklappen").accessibility_label(open ? "Einklappen" : "Aufklappen").role("button")
    .on_click((_e, cx) => { cx.stop_propagation(); action(cx); })
    .child(svg(ICONS + (open ? "chevron-down.svg" : "chevron-right.svg")).size(11).text_color(t.muted));
}
function pageButton(id, back, disabled, action, t) {
  const label = back ? "Vorherige Seite" : "Nächste Seite";
  return h_flex().id(id).w(26).h(26).items_center().justify_center().cursor_pointer().disabled(disabled)
    .opacity(disabled ? 0.4 : 1).tooltip(label).accessibility_label(label).role("button").on_click((_e, cx) => action(cx))
    .child(svg(ICONS + (back ? "chevron-left.svg" : "chevron-right.svg")).size(11).text_color(t.muted));
}
function rowVisible(row, host, archive) {
  const visited = new Set();
  for (let id = row.session.parent_id; id && !visited.has(id); id = host.byId.get(id)?.session.parent_id) {
    visited.add(id);
    if (host.childrenFolded.has(id) || (archive && !host.historyExpanded.has(id))) return false;
  }
  return true;
}
function agentRow(row, host, current, t, project, archive) {
  const s = row.session, state = statusOf(s), chosen = current?.session.id === s.id;
  const hasChildren = host.parents.has(s.id);
  const open = !host.childrenFolded.has(s.id) && (!archive || host.historyExpanded.has(s.id));
  const toggle = cx => {
    if (archive) { host.childrenFolded.delete(s.id); if (!host.historyExpanded.delete(s.id)) host.historyExpanded.add(s.id); }
    else if (!host.childrenFolded.delete(s.id)) host.childrenFolded.add(s.id);
    cx.notify();
  };
  const program = s.source?.CodexApp ? "Codex App" : ({ codex: "Codex", claude: "Claude" })[s.program] ?? s.program;
  const title = s.project !== project ? `${s.project} · ${s.title}` : s.title;
  const command = s.phase_history?.at(-1)?.command;
  const phase = row.depth ? commandText(s) : phaseText(s.act_phase);
  const pillTip = row.depth ? `ACT: ${command ?? "kein ACT-Befehl"}` : `ACT: ${s.act_phase ?? "unbekannt"}`;
  const summary = `${s.title} · ${s.project} · ${program} · ${state.label} · ACT: ${s.act_phase ?? "unbekannt"}`;
  const line = h_flex().w_full().min_w(0).h(row.depth ? 27 : 32).items_center().gap(5)
    .child(hasChildren ? disclosure(s.id, open, toggle, t) : div().w(20).flex_none())
    .child(mark(state, t, s.id))
    .child(div().min_w(0).flex_1().truncate().font_medium().text_size(row.depth ? 12 : 13).child(title))
    .child(div().w(program === "Codex App" ? 64 : 44).flex_none().text_size(11).text_color(t.muted).truncate().child(program))
    .child(div().w(96).flex_none().px(5).py(2).rounded(4).when(Boolean(phase), el => el.bg(t.raised))
      .text_size(11).text_color(t.muted).truncate().tooltip(pillTip).child(phase));
  const selectedFill = /^#[0-9a-f]{6}$/i.test(t.ansi4) ? t.ansi4 + "18" : t.selected;
  const root = v_flex().id("agent-" + s.id).w_full().min_w(0).pl(22 + Math.min(row.depth, 4) * 12).pr(7)
    .rounded(4).bg(chosen ? selectedFill : t.chrome).text_color(t.foreground).cursor_pointer()
    .tooltip(summary).accessibility_label(summary).role("button")
    .hover(el => el.bg(chosen ? selectedFill : t.hover)).on_click((_e, cx) => host.choose(row, cx)).child(line);
  const reason = s.question || s.problem;
  if (reason) root.child(div().pl(41).pb(4).text_size(11).truncate().text_color(t[state.color] ?? t.muted).child(reason.split("\n")[0]));
  host.shownRows.push(row);
  return root;
}
function header(id, title, count, status, open, action, t) {
  return h_flex().id(id).w_full().min_w(0).h(34).gap(5).px(5).items_center().cursor_pointer()
    .text_color(t.foreground).accessibility_label(`${title}, ${count} Hauptagenten`).role("button")
    .hover(el => el.bg(t.hover)).on_click((_e, cx) => action(cx))
    .child(disclosure(id, open, action, t)).child(mark(status, t, id))
    .child(div().min_w(0).flex_1().truncate().font_semibold().text_size(13).child(title))
    .child(div().text_size(11).text_color(t.muted).child(String(count)));
}
function savedSection(list, rows, key, label, host, current, t) {
  if (!rows.length) return;
  const open = !host.folded.has(key);
  const status = key === "Done" ? statusOf({ task_complete: true }) : { icon: "folder", color: "dim", label: "Gespeicherte Unterhaltung, Laufzeit unbekannt" };
  const section = v_flex().w_full().min_w(0).mt(8).border_t_1().border_color(t.border).pt(3)
    .child(header("section-" + key, label, rows.filter(r => !r.depth).length, status, open, cx => host.fold(key, cx), t));
  if (open) {
    const page = pageFamilies(rows, host.pages[key] ?? 0, 8);
    for (const row of page.rows) if (rowVisible(row, host, true)) section.child(agentRow(row, host, current, t, null, true));
    if (page.pages > 1) section.child(h_flex().px(8).h(27).gap(8).items_center()
      .child(pageButton("previous-" + key, true, page.page === 0, cx => { host.pages[key] = page.page - 1; cx.notify(); }, t))
      .child(div().text_size(11).text_color(t.muted).child(`Seite ${page.page + 1} / ${page.pages}`))
      .child(pageButton("next-" + key, false, page.page === page.pages - 1, cx => { host.pages[key] = page.page + 1; cx.notify(); }, t)));
  }
  list.child(section);
}
export function renderProjectTree(host, snapshot, project, showHidden, current, t) {
  if (host.treeSource !== snapshot || host.treeProject !== project || host.treeHidden !== showHidden) {
    host.tree = projectTree(snapshot.rows, { project, showHidden });
    host.parents = new Set(snapshot.rows.map(r => r.session.parent_id).filter(Boolean));
    host.byId = new Map(snapshot.rows.map(r => [r.session.id, r]));
    host.treeSource = snapshot; host.treeProject = project; host.treeHidden = showHidden;
  }
  const tree = host.tree;
  host.shownRows = [];
  const list = v_flex().id("project-tree").min_w(0).min_h(0).flex_1().overflow_y_scrollbar().px(7).gap(1);
  for (const group of tree.projects) {
    const open = !host.projectFolded.has(group.name);
    const block = v_flex().w_full().min_w(0).pb(4)
      .child(header("project-" + group.name, group.name === "Unknown" ? "Unbekanntes Projekt" : group.name, group.count, group.status, open,
        cx => host.foldProject(group.name, cx), t));
    if (open) for (const row of group.rows) if (rowVisible(row, host, false)) block.child(agentRow(row, host, current, t, group.name, false));
    list.child(block);
  }
  if (!tree.projects.length && snapshot.rows.length) list.child(div().px(10).py(12).text_size(12).text_color(t.muted).child("Keine aktuellen Sitzungen in diesem Projekt."));
  savedSection(list, tree.done, "Done", "Fertig", host, current, t);
  savedSection(list, tree.saved, "Saved", "Gespeicherte Unterhaltungen", host, current, t);
  return list;
}
