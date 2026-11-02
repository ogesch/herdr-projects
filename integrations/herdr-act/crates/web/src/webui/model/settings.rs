//! The settings for the web board: every work setting of the open task (or automation) as a row the page can change.
//! The rows are the terminal board's settings page (board/update/edit.rs): the same settings, in the same order, each
//! with its plain label, what it means, its value, whether it is set for this task only, and what it may be. Nothing is decided
//! here: a change goes back through the board's own save (webui/web.rs: set_setting).
use crate::board::state::{Editing, State};
use crate::board::update::edit::{rows, value_say, words_of};
use crate::json::*;
use crate::store::settings_list::entry_of;

/// `auto`: `r` is an automation. None for a row that has no settings (a message that waits for the Coordinator).
pub fn settings_of(s: &mut State, r: &Obj, auto: bool) -> Value {
    if !auto && !r.has("title") { return Value::Null; }
    let whose = if auto { "this automation" } else { "this task" };
    // An automation's are every work setting its next task will have, as it is; a task's are its settings page's.
    let list: Vec<Obj> = if auto { r.at("shown").arr("settings").iter().filter_map(|x| x.as_object().cloned()).collect() } else {
        let was = s.editing.replace(Editing { id: r.t("id"), ..Default::default() });
        let l = rows(s);
        s.editing = was;
        l
    };
    let rows: Vec<Value> = list.iter().map(|x| {
        let e = entry_of(x.s("name"));
        let words = words_of(x.s("name"));
        let from = [x.t("note"), x.t("later")].into_iter().filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" · ");
        json!({
            "name": x.s("name"), "label": e.map(|e| e.label.to_string()).filter(|l| !l.is_empty()).unwrap_or_else(|| x.t("name")), "means": e.map(|e| e.means.to_string()).unwrap_or_default(), "own": x.b("own"),
            "value": crate::re!(r"\s*\n\s*").replace_all(&value_say(x.at("value")), " / "),
            // What a reader must know of it: why it does nothing here, or that it counts later. Never where it comes from.
            "from": from,
            // One of a few words: the page offers them. Anything else is typed, and `hint` says what it may be.
            "words": words.as_ref().map(|ws| ws.iter().map(|w| json!({ "value": text(w), "say": value_say(w), "now": w == x.at("value") })).collect::<Vec<_>>()),
            "hint": if words.is_some() { String::new() } else { crate::board::view::panel::page::value_words(x) },
        })
    }).collect();
    // Read-only: an archived task, and another machine's automation whose machine did not send its settings.
    let can_set = !r.b("archivedAt") && (!auto || crate::board::update::autos::can_set(r));
    // `show`: counts up each time the board is asked for this part by its key, so the page opens it and goes there.
    json!({ "id": if auto { r.t("row") } else { r.t("id") }, "whose": whose, "canSet": can_set, "show": s.web.show_settings, "rows": rows })
}
