// Quiet project tree: selectable project headings, one status mark per agent, program icon, right-aligned ACT step.
import { div, svg, image } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { pageFamilies, families, programOf } from "./model.js";
import { projectTree, currentRows, branchGuides, statusOf, phaseText, commandText } from "./tree-model.js";

const ICONS = "mods/act-overview/icons/";
function mark(status, t, id, size = 12) {
  return h_flex().id("status-" + id).w(16).h(20).flex_none().items_center().justify_center()
    .tooltip(status.label).accessibility_label(status.label)
    .child(svg(ICONS + status.icon + ".svg").size(size).text_color(t[status.color] ?? t.dim));
}
function disclosure(id, open, action, t) {
  return h_flex().id("disclose-" + id).w(18).h(24).flex_none().items_center().justify_center().cursor_pointer()
    .tooltip(open ? "Einklappen" : "Aufklappen").accessibility_label(open ? "Einklappen" : "Aufklappen").role("button")
    .on_click((_e, cx) => { cx.stop_propagation(); action(cx); })
    .child(svg(ICONS + (open ? "chevron-down.svg" : "chevron-right.svg")).size(10).text_color(t.dim));
}
function pageButton(id, back, disabled, action, t) {
  const label = back ? "Vorherige Seite" : "Nächste Seite";
  return h_flex().id(id).w(24).h(24).items_center().justify_center().cursor_pointer().disabled(disabled)
    .opacity(disabled ? 0.4 : 1).tooltip(label).accessibility_label(label).role("button").on_click((_e, cx) => action(cx))
    .child(svg(ICONS + (back ? "chevron-left.svg" : "chevron-right.svg")).size(10).text_color(t.muted));
}
function rowVisible(row, host, archive) {
  const visited = new Set();
  for (let id = row.session.parent_id; id && !visited.has(id); id = host.byId.get(id)?.session.parent_id) {
    visited.add(id);
    if (host.childrenFolded.has(id) || (archive && !host.historyExpanded.has(id))) return false;
  }
  return true;
}
function treeGuide(kind, t) {
  const column = div().w(14).flex_none().relative();
  if (kind !== "blank") column.child(div().absolute().left(9).top(0).w(1).bg(t.border)
    .when(kind === "last", el => el.h(13)).when(kind !== "last", el => el.bottom(0)));
  if (kind === "branch" || kind === "last") column.child(div().absolute().left(9).top(13).w(5).h(1).bg(t.border));
  return column;
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
  const program = programOf(s), child = row.depth > 0, done = state.kind === "done";
  const title = s.project !== project ? `${s.project} · ${s.title}` : s.title;
  const phase = child ? commandText(s) : phaseText(s.act_phase);
  const known = phase !== "Unbekannt";
  const phaseTip = child ? `ACT-Befehl: ${s.phase_history?.at(-1)?.command ?? "unbekannt"}` : `ACT-Schritt: ${s.act_phase ?? "unbekannt"}`;
  const summary = `${s.title} · ${s.project} · ${program.name} · ${state.label} · ACT: ${s.act_phase ?? "unbekannt"}`;
  const line = h_flex().w_full().min_w(0).h(child ? 26 : 30).items_center().gap(5)
    .child(hasChildren ? disclosure(s.id, open, toggle, t) : div().w(18).flex_none())
    .child(mark(state, t, s.id))
    .child(div().min_w(0).flex_1().truncate().text_size(child ? 12 : 13).when(!child, el => el.font_medium())
      .text_color(child || done ? t.muted : t.foreground).child(title))
    .child(h_flex().id("program-" + s.id).w(18).h(20).flex_none().items_center().justify_center()
      .tooltip(program.name).accessibility_label(program.name).child(image(program.icon).size(13)))
    .child(div().id("phase-" + s.id).w(80).flex_none().text_right().text_size(11).truncate()
      .text_color(known ? t.muted : t.dim).tooltip(phaseTip).child(phase));
  const content = v_flex().min_w(0).flex_1().child(line);
  const reason = s.question || s.problem;
  if (reason) content.child(div().pl(39).pb(5).text_size(11.5).truncate().text_color(t[state.color] ?? t.muted).child(reason.split("\n")[0]));
  const root = h_flex().id("agent-" + s.id).w_full().min_w(0).items_stretch().pl(4).pr(8)
    .rounded(5).bg(chosen ? t.selected : t.chrome).text_color(t.foreground).cursor_pointer()
    .tooltip(summary).accessibility_label(summary).role("button")
    .hover(el => el.bg(chosen ? t.selected : t.hover)).on_click((_e, cx) => host.choose(row, cx))
    .children((host.guides.get(s.id) ?? []).map(kind => treeGuide(kind, t))).child(content);
  host.shownRows.push(row);
  return root;
}
// The chevron folds; the name selects the project so a new agent can start there without picking an agent first.
function projectHeader(group, open, chosen, host, t) {
  const name = group.name === "Unknown" ? "Unbekanntes Projekt" : group.name;
  const selectable = group.name !== "Unknown" && Boolean(group.path);
  const label = `${name}, ${group.count} Hauptagenten${selectable ? ", Projekt auswählen" : ""}`;
  return h_flex().id("project-" + group.name).w_full().min_w(0).h(28).mt(4).pr(8).gap(4).items_center().rounded(5)
    .bg(chosen ? t.selected : t.chrome).accessibility_label(label).role("button")
    .when(selectable, el => el.cursor_pointer().tooltip(`${group.path}\nAuswählen, um hier einen neuen Agenten zu starten`)
      .hover(s => s.bg(chosen ? t.selected : t.hover)).on_click((_e, cx) => host.selectProject(group, cx)))
    .child(disclosure("project-" + group.name, open, cx => host.foldProject(group.name, cx), t))
    .child(div().min_w(0).flex_1().truncate().text_size(11.5).font_semibold().text_color(chosen ? t.foreground : t.muted).child(name))
    .when(!open, el => el.child(mark(group.status, t, "project-" + group.name, 10)))
    .child(div().text_size(11).text_color(t.dim).child(String(group.count)));
}
function savedSection(list, rows, host, current, t) {
  if (!rows.length) return;
  const key = "Saved", open = !host.folded.has(key);
  const head = h_flex().id("section-" + key).w_full().min_w(0).h(28).pr(8).gap(4).items_center().rounded(5).cursor_pointer()
    .accessibility_label(`Gespeicherte Unterhaltungen, ${rows.filter(r => !r.depth).length}`).role("button")
    .hover(el => el.bg(t.hover)).on_click((_e, cx) => host.fold(key, cx))
    .child(disclosure("section-" + key, open, cx => host.fold(key, cx), t))
    .child(svg(ICONS + "folder.svg").size(11).text_color(t.dim))
    .child(div().min_w(0).flex_1().truncate().pl(4).text_size(11.5).font_semibold().text_color(t.muted).child("Gespeicherte Unterhaltungen"))
    .child(div().text_size(11).text_color(t.dim).child(String(rows.filter(r => !r.depth).length)));
  const section = v_flex().w_full().min_w(0).mt(6).border_t(1).border_color(t.border).pt(4).child(head);
  if (open) {
    const page = pageFamilies(rows, host.pages[key] ?? 0, 8);
    const visible = page.rows.filter(row => rowVisible(row, host, true));
    host.guides = branchGuides(visible);
    for (const row of visible) section.child(agentRow(row, host, current, t, null, true));
    if (page.pages > 1) section.child(h_flex().pl(22).h(26).gap(8).items_center()
      .child(pageButton("previous-" + key, true, page.page === 0, cx => { host.pages[key] = page.page - 1; cx.notify(); }, t))
      .child(div().text_size(11).text_color(t.muted).child(`Seite ${page.page + 1} / ${page.pages}`))
      .child(pageButton("next-" + key, false, page.page === page.pages - 1, cx => { host.pages[key] = page.page + 1; cx.notify(); }, t)));
  }
  list.child(section);
}
export function renderProjectTree(host, snapshot, project, showHidden, current, t, chosenProject = "") {
  if (host.treeSource !== snapshot || host.treeProject !== project || host.treeHidden !== showHidden) {
    host.tree = projectTree(snapshot.rows, { project, showHidden });
    host.parents = new Set(snapshot.rows.map(r => r.session.parent_id).filter(Boolean));
    host.byId = new Map(snapshot.rows.map(r => [r.session.id, r]));
    host.treeSource = snapshot; host.treeProject = project; host.treeHidden = showHidden;
  }
  const tree = host.tree;
  host.shownRows = [];
  host.parents = new Set(tree.saved.map(row => row.session.parent_id).filter(Boolean));
  const list = v_flex().id("project-tree").min_w(0).min_h(0).flex_1().overflow_y_scrollbar().px(6).pb(8).gap(1);
  for (const group of tree.projects) {
    const open = !host.projectFolded.has(group.name);
    const block = v_flex().w_full().min_w(0).child(projectHeader(group, open, chosenProject === group.name, host, t));
    const active = currentRows(group.rows), historyKey = "project-history-" + group.name;
    const history = host.historyExpanded.has(historyKey), count = group.rows.length - active.length;
    const rows = history ? group.rows : active;
    for (const row of rows) if (row.session.parent_id) host.parents.add(row.session.parent_id);
    if (open && count) block.child(h_flex().id(historyKey).h(26).pl(18).gap(4).items_center().cursor_pointer()
      .accessibility_label(`Verlauf (${count})`).role("button").on_click((_e, cx) => {
        if (!host.historyExpanded.delete(historyKey)) host.historyExpanded.add(historyKey);
        cx.notify();
      }).child(h_flex().w(18).h(24).flex_none().items_center().justify_center()
        .child(svg(ICONS + (history ? "chevron-down.svg" : "chevron-right.svg")).size(10).text_color(t.dim)))
      .child(div().text_size(11).text_color(t.muted).child(`Verlauf (${count})`)));
    if (open) for (const family of families(rows)) {
      const visible = family.filter(row => rowVisible(row, host, false));
      host.guides = branchGuides(visible);
      for (const row of visible) block.child(agentRow(row, host, current, t, group.name, false));
    }
    list.child(block);
  }
  if (!tree.projects.length && snapshot.rows.length) list.child(div().px(10).py(12).text_size(12).text_color(t.muted).child("Keine aktuellen Sitzungen in diesem Projekt."));
  savedSection(list, tree.saved, host, current, t);
  return list;
}
