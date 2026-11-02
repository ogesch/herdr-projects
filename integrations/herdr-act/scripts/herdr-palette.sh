#!/bin/sh
# herdr's own colour code, taken from herdr's source at one version: its Palette (src/app/state.rs: every built-in
# theme's colours), the names it accepts for a theme and how it reads a colour (src/config/theme.rs). The board draws
# with these exactly as herdr does, so nothing is copied by hand. Run it with a herdr version to take that one's:
#   sh scripts/herdr-palette.sh v0.9.3
# It writes rs/herdr/palette.rs. herdr is Apache-2.0 (https://github.com/herdrdev/herdr); the file says so.
set -eu
tag=${1:-v0.9.3}
base="https://raw.githubusercontent.com/herdrdev/herdr/$tag/src"
out="$(dirname "$0")/../rs/herdr/palette.rs"
tmp=$(mktemp -d)
curl -fsSL "$base/app/state.rs" -o "$tmp/state.rs"
curl -fsSL "$base/config/theme.rs" -o "$tmp/theme.rs"
{
  echo "//! herdr's own colours: its Palette and every built-in theme, the names it accepts for a theme, and how it reads a"
  echo "//! colour, taken unchanged from herdr $tag (src/app/state.rs and src/config/theme.rs) by scripts/herdr-palette.sh."
  echo "//! Do not edit: run the script with a newer herdr version instead. herdr: Copyright the herdr authors, Apache-2.0."
  echo "#![allow(dead_code, clippy::all)]"
  echo "use ratatui::style::Color;"
  echo
  # The names list and canonical_theme_name, from the top of theme.rs to the end of that function.
  awk '/^pub\(crate\) const|^pub const|^const/ && /THEME/ {p=1} p {print} /^pub\(crate\) fn canonical_theme_name/ {f=1} f && /^}/ {exit}' "$tmp/theme.rs" | sed 's/^pub(crate) /pub /'
  echo
  # parse_color, without its log line (the board has no log there: an unknown colour is cyan, as in herdr).
  awk '/^pub fn parse_color/ {p=1} p {print} p && /^}/ {exit}' "$tmp/theme.rs" | sed '/warn!(/d'
  echo
  # The Palette and its built-in themes, up to from_name included.
  awk '/^pub struct Palette/ {p=1; print "#[derive(Debug, Clone, PartialEq, Eq)]"} p && /Apply custom color overrides/ {exit} p {print}' "$tmp/state.rs" | sed 's/crate::config::canonical_theme_name/canonical_theme_name/'
  echo "}"
} > "$out"
rm -rf "$tmp"
echo "wrote $out from herdr $tag"
