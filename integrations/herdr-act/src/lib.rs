//! herdr-projects: a work board for herdr, one native program. The board's screen (board/), its command line (cli/),
//! the keeper that runs the flow of a task, other machines, and the web Board.
//! `herdr-projects docs --where` prints which file holds what, from the comment at the top of each file.
//! This crate is the command line and the made pages, on the crates below it (sw-web, sw-board, sw-flow, sw-core),
//! whose modules it passes on as its own: everything is `sw::…`, as before the split (docs/crates.md).
pub use sw_web::*;

pub mod cli;
pub mod docs;
