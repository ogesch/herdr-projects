//! The board's words in named inks as the web page reads them: pieces `{ t: text, c: colour name, b: bold, d: dim,
//! href: link }`. The page gives each colour name a colour of its own; nothing here is a colour. A quiet ink (a
//! finished task's mark) is dim on the page.
use crate::json::*;
use crate::shown::ink::{Ink, Inked};

pub fn pieces(x: &Inked) -> Vec<Value> {
    let mut out: Vec<(String, &'static str, bool, bool, String)> = vec![];
    for p in &x.0 {
        if p.t.is_empty() { continue; }
        let c = if p.c == Ink::Plain { "" } else { p.c.name() };
        let href = p.link.clone().filter(|l| l.starts_with("http://") || l.starts_with("https://")).unwrap_or_default();
        let (b, d) = (p.bold, p.dim || p.quiet);
        if let Some(last) = out.last_mut() { if last.1 == c && last.2 == b && last.3 == d && last.4 == href { last.0.push_str(&p.t); continue; } }
        out.push((p.t.clone(), c, b, d, href));
    }
    out.into_iter().map(|(t, c, b, d, href)| {
        let mut o = Obj::new();
        o.put("t", t);
        if !c.is_empty() { o.put("c", c); }
        if b { o.put("b", true); }
        if d { o.put("d", true); }
        if !href.is_empty() { o.put("href", href); }
        Value::Object(o)
    }).collect()
}
/// The first piece (a mark), or a dot where there is none.
pub fn first(x: &Inked) -> Value { pieces(x).into_iter().next().unwrap_or_else(|| json!({ "t": "·" })) }
pub fn plain(x: &Inked) -> String { x.text() }
