// Horizontal session metadata keeps runtime, assignment completion, ACT phase, and ticket distinct.
import { div } from "gpui-kit";
import { h_flex, v_flex, Link } from "gpui-base";
import { runtime, completion, sourceName } from "./model.js";

function badge(id, label, color, t) {
  return div().id(id).flex_none().min_w(0).max_w_full().truncate().tooltip(label).px(8).py(4).rounded(5).text_size(11).font_semibold()
    .bg(/^#[0-9a-f]{6}$/i.test(color) ? color + "18" : t.chrome).text_color(color).child(label);
}

export function sessionSummary(s, t) {
  const complete = completion(s);
  const color = ({ working: t.ansi4, active: t.ansi4, blocked: t.ansi3, waiting: t.ansi3,
    "waiting-parent": t.ansi3, error: t.ansi1 })[s.runtime_status] ?? t.muted;
  const project = `${s.project} · ${s.program} · ${sourceName(s)}`;
  const ticketUrl = /^https?:\/\/\S+$/i.test(s.ticket ?? "") ? s.ticket : null;
  const ticket = ticketUrl ? Link.new("session-ticket").href(ticketUrl).cursor_pointer().tab_index(0)
    .accessibility_label(`Ticket im Browser öffnen: ${ticketUrl}`).hover(el => el.underline()) : div().id("session-ticket");
  return h_flex().id("act-session-summary").min_w(0).flex_wrap().items_center().gap(12)
    .px(10).py(9).rounded(8).bg(t.chrome)
    .child(v_flex().min_w(0).flex_basis(190).flex_grow(1).gap(6)
      .child(div().truncate().tooltip(project).text_color(t.muted).text_size(11).child(project))
      .child(h_flex().flex_wrap().gap(6)
        .child(badge("session-runtime", runtime(s.runtime_status), color, t))
        .child(badge("session-completion", `Auftrag: ${complete === true ? "Fertig" : complete === false ? "Nicht abgeschlossen" : "Unbekannt"}`,
          complete === true ? t.ansi2 : t.muted, t))))
    .child(v_flex().min_w(0).flex_basis(180).gap(6)
      .child(badge("session-phase", `ACT: ${s.act_phase ?? "Unbekannt"}`, s.act_phase ? t.ansi4 : t.muted, t))
      .child(div().truncate().tooltip(s.phase_source ?? "Keine Phasenmeldung").text_color(t.muted).text_size(11)
        .child(s.phase_source ?? "Keine Phasenmeldung")))
    .child(v_flex().min_w(0).flex_basis(240).flex_grow(1).gap(6)
      .child(div().text_size(11).text_color(t.muted).child("Spec/Ticket"))
      .child(ticket.truncate().text_ellipsis_middle().tooltip(s.ticket ?? "Unbekannt")
        .text_color(ticketUrl ? t.ansi4 : t.muted).child(s.ticket ?? "Unbekannt")));
}
