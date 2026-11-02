//! The repo's own rules, checked with the rest: no source file past its size, every one says what it holds (the map
//! is made from those words), a key's chord only in the keys' table, every test file in the one test program, and no
//! made table out of date.
#[test]
fn every_source_file_is_within_its_size_says_what_it_holds_and_every_made_table_is_up_to_date() {
    let bad = sw::docs::guard::problems();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
#[test]
fn the_map_is_made_from_each_files_top_comment_whole_or_for_one_folder() {
    let all = sw::docs::map::where_page("").unwrap();
    assert!(all.contains("## `crates/board/src/board/`") && all.contains("## The pages: `docs/`"), "{all}");
    let one = sw::docs::map::where_page("crates/web/src/webui").unwrap();
    assert!(one.contains("## `crates/web/src/webui/`") && !one.contains("## `crates/board/src/board/`") && !one.contains("The pages"));
    assert!(sw::docs::map::where_page("nowhere").unwrap().contains("No folder of source files is at or under `nowhere`."));
}
#[test]
fn a_test_file_the_one_test_program_does_not_name_is_a_problem() {
    let files: Vec<String> = ["tests/main.rs", "tests/a.rs", "tests/b_more.rs", "tests/common/mod.rs", "src/x.rs"].iter().map(|s| s.to_string()).collect();
    let bad = sw::docs::guard::tests_left_out(&files, "//! The one test program.\nmod common;\nmod a;\n");
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].starts_with("tests/b_more.rs is not in the one test program") && bad[0].contains("add `mod b_more;` to tests/main.rs"), "{}", bad[0]);
}
