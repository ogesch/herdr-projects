// A selected project: its Space is shown and the detail area offers starting a new agent there.
import { div, image } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { shared, setShared, focused, spaces, panes, run, knownAgents } from "herdr";

export const CHOSEN = "herdr-act.chosen-project";

// The Space that already runs this project's agents (the shown one first), else one opened in its folder.
export function projectSpace(group, all = spaces(), paneList = panes()) {
  const tabOf = new Map(paneList.filter(p => p.terminal).map(p => [p.terminal, p.tab]));
  const owner = tab => all.find(space => (space.tabs ?? []).some(t => t.id === tab));
  const used = group.rows.map(r => owner(tabOf.get(r.session.source?.Native?.terminal))).filter(Boolean);
  const shown = focused()?.space;
  return used.find(s => s.id === shown) ?? used[0]
    ?? all.find(s => s.folder === group.path) ?? all.find(s => paneList.some(p => p.cwd === group.path && owner(p.tab)?.id === s.id)) ?? null;
}

export function selectProject(group, cx) {
  const space = projectSpace(group);
  setShared("herdr-act.selected", null);
  setShared(CHOSEN, { name: group.name, path: group.path, space: space?.id ?? null, tab: space?.id === focused()?.space ? focused()?.tab : null,
    count: group.count, open: group.open });
  if (space && space.id !== focused()?.space) run("space.select", { id: space.id });
  cx.notify();
}

// Keeps the choice while its Space and tab stay shown; another tab or Space (a new agent's tab too) ends it.
export function followFocus(focus) {
  const chosen = shared(CHOSEN);
  if (!chosen) return;
  if (chosen.space && focus?.space === chosen.space && !chosen.tab) { setShared(CHOSEN, { ...chosen, tab: focus.tab }); return; }
  if (!chosen.space || focus?.space !== chosen.space || focus?.tab !== chosen.tab) setShared(CHOSEN, null);
}

function start(chosen, agent) {
  const args = { folder: chosen.path, argv: agent.run };
  if (chosen.space) run("tab.new", { ...args, space: chosen.space });
  else run("space.new", args);
}

export function projectDetail(host, chosen, t, cx) {
  if (!host.known && !host.loadingKnown) {
    host.loadingKnown = true;
    cx.spawn(async cx => {
      try { host.known = (await knownAgents()).filter(a => a.installed && a.agent); } catch { host.known = []; }
      host.loadingKnown = false; cx.notify();
    });
  }
  const space = spaces().find(s => s.id === chosen.space);
  const sep = () => div().text_color(t.dim).child("·");
  const agents = (host.known ?? []).slice(0, 4);
  return v_flex().id("act-project").size_full().min_w(0).min_h(0).px(16).pt(10).pb(8).gap(6).bg(t.surface).text_color(t.foreground).text_size(12)
    .child(h_flex().h(28).items_center().gap(8)
      .child(div().min_w(0).flex_1().truncate().font_semibold().text_size(15).child(chosen.name)))
    .child(h_flex().min_w(0).gap(7).items_center().text_color(t.muted)
      .child("Projekt").child(sep()).child(`${chosen.count} Hauptagenten`).child(sep()).child(`${chosen.open} offen`)
      .child(sep()).child(div().min_w(0).truncate().tooltip(chosen.path).child(chosen.path.replace(/^\/Users\/[^/]+/, "~")))
      .child(sep()).child(div().flex_none().child(space ? `Space ${space.name}` : "Noch kein Space")))
    .child(h_flex().min_w(0).gap(4).items_center()
      .child(div().pr(4).text_color(t.muted).child("Neuer Agent hier:"))
      .when(host.loadingKnown && !host.known, el => el.child(div().text_color(t.dim).child("Programme werden gesucht …")))
      .when(Boolean(host.known) && !agents.length, el => el.child(div().text_color(t.dim).child("Kein Agentenprogramm gefunden. Nutze + in der Tab-Leiste.")))
      .children(agents.map(agent => h_flex().id("start-" + agent.key).gap(5).items_center().px(8).py(4).rounded(6).cursor_pointer()
        .role("button").tab_index(0).accessibility_label(`${agent.name} in ${chosen.name} starten`).text_size(12).text_color(t.foreground)
        .hover(s => s.bg(t.hover)).on_click(() => start(chosen, agent))
        .on_key_down((event, cx) => { if (["enter", "space"].includes(event.keystroke)) { cx.stop_propagation(); start(chosen, agent); } })
        .when(Boolean(agent.icon), el => el.child(image(agent.icon).size(13))).child(agent.name))));
}
