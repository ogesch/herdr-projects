// One quiet metadata line: runtime, assignment completion, ACT phase and Spec/Ticket stay separate facts.
import { div, image } from "gpui-kit";
import { h_flex, Link } from "gpui-base";
import { runtime, completion, sourceName, programOf } from "./model.js";

const sep = (t, i) => div().id("summary-sep-" + i).flex_none().text_color(t.dim).child("·");
function fact(id, label, color, t, tip = label) {
  return div().id(id).flex_none().min_w(0).truncate().tooltip(tip).text_color(color).child(label);
}

export function sessionSummary(s, t) {
  const complete = completion(s);
  const color = ({ working: t.ansi4, active: t.ansi4, blocked: t.ansi3, waiting: t.ansi3,
    "waiting-parent": t.ansi3, error: t.ansi1 })[s.runtime_status] ?? t.muted;
  const program = programOf(s);
  const ticketUrl = /^https?:\/\/\S+$/i.test(s.ticket ?? "") ? s.ticket : null;
  const ticket = ticketUrl ? Link.new("session-ticket").href(ticketUrl).cursor_pointer().tab_index(0)
    .accessibility_label(`Ticket im Browser öffnen: ${ticketUrl}`).hover(el => el.underline()) : div().id("session-ticket");
  const items = [
    fact("session-runtime", runtime(s.runtime_status), color, t, `Agentenstatus: ${runtime(s.runtime_status)}`),
    fact("session-completion", `Auftrag: ${complete === true ? "Fertig" : complete === false ? "Nicht abgeschlossen" : "Unbekannt"}`,
      complete === true ? t.ansi2 : t.muted, t),
    fact("session-phase", `ACT: ${s.act_phase ?? "Unbekannt"}`, s.act_phase ? t.foreground : t.dim, t, s.phase_source ?? "Keine Phasenmeldung"),
    h_flex().id("session-ticket-fact").flex_none().min_w(0).gap(4).items_center()
      .child(div().text_color(t.dim).child("Spec/Ticket:"))
      .child(ticket.min_w(0).max_w(260).truncate().text_ellipsis_middle().tooltip(s.ticket ?? "Spec/Ticket unbekannt")
        .text_color(ticketUrl ? t.ansi4 : s.ticket ? t.muted : t.dim).child(s.ticket ?? "Unbekannt")),
    fact("session-project", s.project || "Unbekanntes Projekt", t.muted, t),
    h_flex().id("session-program").flex_none().gap(4).items_center().tooltip(`${program.name} · ${sourceName(s)}`)
      .child(image(program.icon).size(12)).child(div().text_color(t.muted).child(`${program.name} · ${sourceName(s)}`)),
  ];
  if (s.worktree) items.push(fact("session-worktree", `Branch ${s.worktree.branch ?? "losgelöst"}`, t.muted, t, `Worktree: ${s.worktree.path}`));
  return h_flex().id("act-session-summary").min_w(0).flex_wrap().items_center().gap(7).text_size(12)
    .children(items.flatMap((item, i) => i ? [sep(t, i), item] : [item]));
}
