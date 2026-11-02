//! The map of which file holds what, what the first port left out: its title, the scripts' folder with the check's
//! own line, a folder alone with only its heading, and a folder with the folders under it.
fn re(r: &str, s: &str) -> bool { regex::Regex::new(r).unwrap().is_match(s) }

#[test]
fn the_map_has_its_title_the_scripts_and_one_folder_or_a_folder_with_those_under_it() {
    let all = sw::docs::map::where_page("").unwrap();
    assert!(all.starts_with("# Where things live\n"), "{all}");
    assert!(all.contains("\n## `src/`\n") && all.contains("\n## `scripts/`\n"));
    assert!(re(r"\| `check\.sh` \| The one command that runs the tests", &all));
    // (The old map left the tests out, as `ls test` lists them; this one lists `tests/` too, each file by its comment.)
    let view = sw::docs::map::where_page("crates/board/src/board/view/").unwrap();
    let heads: Vec<&str> = view.lines().filter(|l| l.starts_with("## ")).collect();
    assert_eq!(heads, vec!["## `crates/board/src/board/view/`", "## `crates/board/src/board/view/panel/`"], "that folder and those under it, nothing else");
    assert!(re(r"\| `layout\.rs` \| The whole screen", &view));
    assert!(re(r"\n## `crates/board/src/board/`\n[\s\S]*\n## `crates/board/src/board/view/`\n", &sw::docs::map::where_page("crates/board/src/board").unwrap()));
}
