// ACT space rows: show the resolved repository and working folder while preserving native navigation.
import { div, svg } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { spaces, machines, run, theme, shared, setShared, layout } from "herdr";
import { describeSpace } from "./context.js";

function find(node, id) {
  if (node?.id === id) return node;
  for (const child of node?.children ?? []) { const result = find(child, id); if (result) return result; }
}
const narrow = () => (find(layout(), "sidebar")?.size?.px ?? Infinity) < 160;
const rowHeight = () => 44;
const MARK = { ansi6: "#22a99a", ansi3: "#e3814f", ansi4: "#5b8def", ansi5: "#8b7cf6", ansi2: "#3fae6a", ansi1: "#d9649a" };

// A space speaks up only when something in it needs you.
export function glyph(status) {
  return status === "blocked" || status === "done" ? "●" : "·";
}

export function tone(t, status) {
  return t[status] ?? t.unknown;
}


function selectScope(all) {
  setShared("herdr-act.all-spaces", all);
  setShared("herdr-act.project", "");
  setShared("herdr-act.selected", null);
}
function selectSpace(id) {
  selectScope(false);
  run("space.select", { id });
}

function grouped(host, t, collapsed) {
    const rows = host.ordered();
    const draw = (r) => (collapsed ? compact(host, r, t) : row(host, r, t));
    const remote = [...new Set([...rows.map(r => r.w.machine).filter(Boolean), ...host.machines.map(m => m.name)])].sort();
    const loading = m => {
      const layout = machines().find(k => k.name === m)?.layout;
      return layout === "loading" || (!layout && host.machines.some(k => k.name === m && k.state === "connected"))
        || spaces().some(s => (s.machine ?? null) === m && !host.shown.has(s.id));
    };
    const loadingRow = m => div().id(`machine-loading-${m ?? "local"}`).h(28).flex_none().px(8).text_size(12).text_color(t.dim).child(collapsed ? "…" : "Lädt …");
    if (!remote.length) return loading(null) && !rows.length ? [loadingRow(null)] : rows.map(draw);
    const machineNames = [null, ...remote];
    return machineNames.flatMap((m) => {
      const mine = rows.filter((r) => (r.w.machine ?? null) === m);
      const machine = host.machines.find(k => k.name === m);
      const connected = !m || machine?.state === "connected";
      const folded = host.folded.has(m);
      const head = h_flex().id(`machine-${m ?? "local"}`).h(26).flex_none().px(collapsed ? 2 : 8).gap(7).items_center().text_size(11).text_color(t.dim)
        .cursor_pointer().hover(s => s.bg(t.hover))
        .on_click((_e, cx) => {
          folded ? host.folded.delete(m) : host.folded.add(m);
          localStorage.setItem("machines.folded", JSON.stringify([...host.folded]));
          cx.notify();
        })
        .child(svg(`mods/workspace-chrome/${m ? "server" : "monitor"}.svg`).size(13).flex_none())
        .when(!collapsed, el => el.child(div().flex_1().truncate().child(m ?? "Dieser Computer"))
          .when(!connected, el => el.child(div().flex_none().child((machine?.state ?? "asleep").replace(/_/g, " "))))
          .when(connected, el => el.child(div().id(`machine-add-${m ?? "local"}`).flex_none().px(4).rounded(4).hover(s => s.bg(t.hover).text_color(t.foreground))
            .on_click((_e, cx) => { cx.stop_propagation(); run("space.new", m ? { machine: m } : {}); }).child("+")))
          .child(div().size(6).rounded(3).flex_none().bg(connected ? t.ansi2 : t.dim))
          .child(svg(`mods/workspace-chrome/chevron-${folded ? "right" : "down"}.svg`).size(11).flex_none()));
      const empty = h_flex().id(`machine-new-${m ?? "local"}`).h(28).flex_none().px(8).gap(7).items_center().rounded(7)
        .text_size(12).text_color(t.dim).hover(s => s.bg(t.hover).text_color(t.foreground))
        .on_click(() => run("space.new", m ? { machine: m } : {}))
        .child(svg("mods/workspace-chrome/plus.svg").size(12)).when(!collapsed, el => el.child("Neuer Space hier"));
      return [head, ...(folded ? [] : mine.length ? mine.map(draw) : [loading(m) ? loadingRow(m) : empty])];
    });
  }

  // Only what's unusual, after the name: "herdr \u00b7 spike \u21912 \u21931". The repo or
  // branch goes when the name already says it (a worktree is named after its branch),
  // and master/main go: nearly every space is on one.
function gitTail(g, t) {
  return h_flex().flex_1().gap(5).justify_end().text_size(11.5).text_color(t.dim)
    .when(g.ahead > 0, el => el.child(div().text_color(t.ansi2).child(`↑${g.ahead}`)))
    .when(g.behind > 0, el => el.child(div().text_color(t.ansi1).child(`↓${g.behind}`)));
}

function row(host, item, t) {
    const { w, i, g, tree } = item, details = describeSpace(item);
    const on = w.active && shared("herdr-act.all-spaces") !== true;
    const colors = shared("space.colors") ?? {};
    const color = colors[w.id] ?? "ansi6";
    const fill = color.startsWith("#") ? color : (MARK[color] ?? t[color]);
    return v_flex()
      .relative()
      .id(`space-${w.id}`).role("button").tab_index(0).track_focus(host.focus.get(w.id))
      .accessibility_label(`${w.name}, ${details.repository}, Arbeitsordner ${details.folder}`)
      .tooltip(`${details.repository}\n${details.folder}`)
      .on_key_down((event, cx) => {
        if (["enter", "space"].includes(event.keystroke)) { cx.stop_propagation(); selectSpace(w.id); }
      })
      .px(8)
      .h(rowHeight({ tree }))
      .flex_none()
      .justify_center()
      .rounded(8)
      .when(on, (el) => el.bg(t.selected))
      .when(!on, (el) => el.hover((s) => s.bg(t.hover)))
      .when(host.pressed === i && host.over !== i, (el) => el.opacity(0.5))
      .when(host.pressed !== null && host.over === i && host.pressed !== i, (el) => el.child(
        div().absolute().left(0).right(0).h(2).bg(t.accent)
          .when(host.pressed < i, line => line.bottom(0))
          .when(host.pressed > i, line => line.top(0)),
      ))
      .on_mouse_down("left", () => {
        host.pressed = i;
        host.over = i;
      })
      .on_mouse_move((_e, cx) => {
        const source = spaces()[host.pressed];
        const over = source && (source.machine ?? null) === (w.machine ?? null) ? i : null;
        if (host.pressed !== null && host.over !== over) {
          host.over = over;
          cx.notify();
        }
      })
      .on_mouse_up("left", (_e, cx) => {
        const from = host.pressed;
        const over = host.over;
        host.pressed = null;
        host.over = null;
        if (from !== null && over === i && from !== i) run("space.move", { id: spaces()[from].id, index: i + 1 });
        else if (from === i) selectSpace(w.id);
        cx.notify();
      })
      .on_mouse_down("right", () => host.menu(w))
      .child(
        h_flex()
          .gap(9)
          .items_center()
          .when(Boolean(tree), el => el.pl(22).child(svg("mods/workspace-chrome/branch.svg").size(13).text_color(t.dim)))
          .when(!tree, el => el.child(h_flex().size(18).flex_none().rounded(5).bg(fill).items_center().justify_center().text_size(10.5).font_bold().text_color("#ffffff").child([...w.name][0]?.toUpperCase() ?? "?")))
          .child(div().flex_shrink(0).max_w(150).truncate().text_color(t.foreground).child(w.name))
          .child(g ? gitTail(g, t) : div().flex_1()))
        .child(div().pl(tree ? 44 : 27).text_size(11).text_color(t.dim).truncate().child(details.line));
  }

function compact(host, { w, i }, t) {
    const on = w.active && shared("herdr-act.all-spaces") !== true;
    return h_flex()
      .id(`space-${w.id}`)
      .h(26)
      .flex_none()
      .justify_center()
      .gap(4)
      .when(on, (el) => el.bg(t.raised))
      .hover((s) => s.bg(t.hover))
      .on_click(() => selectSpace(w.id)).role("button").tab_index(0)
      .child(div().text_color(on ? t.foreground : t.muted).child(String(i + 1)))
      .child(div().text_color(tone(t, w.status)).child(glyph(w.status)));
  }

export function renderSpaces(host) {
    const t = theme(), all = shared("herdr-act.all-spaces") === true;
    const collapsed = narrow();
    const palette = ["ansi6", "ansi3", "ansi4", "ansi5", "ansi2", "ansi1"];
    const sp = spaces();
    const ids = new Set(sp.map(p => p.id));
    const kept = Object.fromEntries(Object.entries(shared("space.colors") ?? {}).filter(([id]) => ids.has(id)));
    let changed = Object.keys(kept).length !== Object.keys(shared("space.colors") ?? {}).length;
    for (const p of sp) {
      if (kept[p.id] || p.name === "~") continue;
      const uses = c => Object.values(kept).filter(v => v === c).length;
      kept[p.id] = palette.reduce((best, c) => uses(c) < uses(best) ? c : best);
      changed = true;
    }
    if (changed) setShared("space.colors", kept);
    return v_flex().size_full().min_h(0).bg(t.chrome).text_size(13)
      .child(h_flex().h(31).pl(16).pr(10).items_center().justify_between().text_size(11.5).text_color(t.dim)
        .when(!collapsed, el => el.child(div().font_semibold().text_color(t.muted).child("Spaces")))
        .child(div().id("space-all").px(8).py(4).rounded(6).cursor_pointer().role("button").tab_index(0).track_focus(host.allFocus)
          .accessibility_label("Alle Spaces").tooltip("Alle Agenten und gespeicherten Gespräche anzeigen")
          .hover(el => el.bg(t.hover).text_color(t.foreground)).when(all, el => el.bg(t.selected).text_color(t.foreground))
          .on_click(() => selectScope(true))
          .on_key_down((event, cx) => { if (["enter", "space"].includes(event.keystroke)) { cx.stop_propagation(); selectScope(true); } })
          .child(collapsed ? "Alle" : "Alle Spaces"))
        .child(div().id("space-new").size(26).flex_none().flex().items_center().justify_center().rounded(6).hover(s => s.bg(t.hover).text_color(t.foreground))
          .on_click(() => run("space.new")).child(svg("mods/workspace-chrome/plus.svg").size(14))))
      .child(v_flex().flex_1().min_h(0).px(8).pb(14).overflow_y_scrollbar().mode("always")
        .on_mouse_up("left", (_e, cx) => { host.pressed = null; cx.notify(); })
        .children(grouped(host, t, collapsed)));
  }
