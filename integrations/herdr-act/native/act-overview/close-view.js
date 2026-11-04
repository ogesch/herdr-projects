// Close an agent from its row's context menu or its details: the menu, the confirmation, and the close itself.
import { div } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { theme, props, shared, exec, openOverlay, closeOverlay } from "herdr";
import { closeSession, closeEffect } from "./lifecycle.js";

export function openClose(session) {
  openOverlay({ mod: "act-overview", kind: "dialog", width: 460, props: { mode: "close", session } });
}
export function openMenu(session) {
  openOverlay({ mod: "act-overview", kind: "popup", width: 200, height: 80, props: { mode: "menu", session } });
}
function item(id, label, action, t, color) {
  return div().id(id).px(10).py(6).rounded(5).text_size(12.5).text_color(color ?? t.foreground).cursor_pointer()
    .role("menu_item").accessibility_label(label).hover(s => s.bg(t.hover)).on_click(action).child(label);
}
export function agentMenu() {
  const t = theme(), s = props().session, overlay = props().overlay;
  return v_flex().w(200).p(4).gap(1).bg(t.popover).text_color(t.foreground)
    .child(item("menu-close", s.parent_id ? "Unteragent schließen …" : "Agent schließen …", () => { closeOverlay(overlay); openClose(s); }, t, t.ansi1));
}
// `button` and `done` come from the overview so the dialog looks and resets like the rest of it.
export function closeDialog(host, button, done, binary) {
  const t = theme(), s = props().session, rows = (shared("herdr-act.snapshot") ?? { rows: [] }).rows;
  const confirm = (_e, cx) => {
    if (host.closing) return;
    host.closing = true; host.error = ""; cx.notify();
    const overlay = props().overlay;
    cx.spawn(async cx => {
      try { await closeSession(s, exec, binary); done(); closeOverlay(overlay); }
      catch (e) { host.error = e.message ?? String(e); }
      host.closing = false; cx.notify();
    });
  };
  return v_flex().w(460).p(20).gap(12).bg(t.popover).text_color(t.foreground)
    .child(div().font_semibold().child(s.parent_id ? "Unteragent schließen?" : "Agent schließen?"))
    .child(div().text_size(13).child(s.title))
    .child(div().text_size(12).text_color(t.muted).child(closeEffect(s, rows)))
    .when(Boolean(host.error), el => el.child(div().text_size(12).text_color(t.ansi1).child(host.error)))
    .child(h_flex().justify_end().gap(8)
      .child(button("cancel", "Abbrechen", () => closeOverlay(props().overlay), t).disabled(Boolean(host.closing)))
      .child(button("confirm-close", host.closing ? "Wird geschlossen …" : "Schließen", confirm, t)
        .text_color(t.ansi1).disabled(Boolean(host.closing))));
}
