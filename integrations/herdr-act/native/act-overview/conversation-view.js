// Read-only child conversation content and its loading, unavailable, and failure states.
import { div } from "gpui-kit";
import { v_flex } from "gpui-base";

export function conversationView(id, reader, t, button, toggleAssignment) {
  const current = reader?.id === id, data = current ? reader.data : null;
  const view = v_flex().id("conversation-" + id).min_w(0).max_w(760).pt(14).gap(18);
  if (current && reader.error) view.child(div().text_size(14).text_color(t.ansi1).child(reader.error));
  if (!data) return view.child(div().text_size(14).text_color(t.muted).child(current && reader.error ? "Über „Verlauf aktualisieren“ kannst du es erneut versuchen." : "Gesprächsverlauf wird geladen …"));
  if (!data.available) return view.child(div().text_size(14).text_color(t.muted).child("Für diesen Unteragenten ist kein gespeicherter Verlauf verfügbar."));
  if (data.assignment) {
    const chars = Array.from(data.assignment), long = chars.length > 600;
    const text = long && !reader.assignmentExpanded ? chars.slice(0, 600).join("") + " …" : data.assignment;
    const assignment = v_flex().min_w(0).gap(6)
      .child(div().font_semibold().text_size(15).child("Auftrag"))
      .child(div().text_size(15).child(text));
    if (long) assignment.child(button("expand-assignment", reader.assignmentExpanded ? "Auftrag einklappen" : "Auftrag ausklappen", (_e, cx) => toggleAssignment(cx)));
    view.child(assignment);
  } else view.child(div().text_size(14).text_color(t.muted).child("Der ursprüngliche Auftrag ist hier nicht als lesbarer Text gespeichert."));
  view.child(div().font_semibold().text_size(15).child("Gesprächsverlauf"));
  if (reader.loading) view.child(div().text_size(14).text_color(t.muted).child("Verlauf wird aktualisiert …"));
  if (!data.messages.length) view.child(div().text_size(14).text_color(t.muted).child("Der Unteragent hat bisher keine lesbaren Nachrichten gespeichert."));
  view.children(data.messages.map((message, i) => v_flex().id("message-" + i).min_w(0).gap(6)
    .child(div().font_semibold().text_size(14).text_color(t.muted).child(message.role === "user" ? "Hauptagent" : "Unteragent"))
    .child(div().text_size(15).child(message.text))));
  if (data.truncated) view.child(div().text_size(14).text_color(t.muted).child("Auszug aus den letzten 80 Nachrichten; sehr lange Texte sind gekürzt."));
  return view;
}
