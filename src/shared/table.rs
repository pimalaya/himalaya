//! # Table
//!
//! Maps the `table.preset` string onto a comfy-table [`TableStyle`], and
//! makes the strings a server or a sender controls safe to put in a cell.
//!
//! comfy-table v8 dropped the positional preset string for a typed
//! builder, but the option keeps accepting the v7 spelling so existing
//! configurations stay valid.

use std::borrow::Cow;

use pimalaya_cli::table::{ContentLineStyle, LineStyle, TableStyle};

/// Default preset: full UTF-8 borders with no divider between rows, the
/// v7 `UTF8_FULL_CONDENSED`.
pub const DEFAULT_PRESET: &str = "││──╞═╪╡┆    ┬┴┌┐└┘";

/// Number of table components a preset string can style.
const COMPONENTS: usize = 19;

/// Maps a v7 positional preset string onto a [`TableStyle`].
///
/// Each character styles one component, in the order of the v7
/// `TableComponent` enum:
///
/// ```text
///  0 left border           7 right header intersection   14 bottom border intersections
///  1 right border          8 vertical lines              15 top left corner
///  2 top border            9 horizontal lines            16 top right corner
///  3 bottom border        10 middle intersections        17 bottom left corner
///  4 left header inters.  11 left border intersections   18 bottom right corner
///  5 header lines         12 right border intersections
///  6 middle header inters. 13 top border intersections
/// ```
///
/// A space leaves a component undrawn, and so does one a short string
/// leaves out, both matching v7 where an unset component rendered blank.
/// Characters past the nineteenth are ignored.
pub fn style_from_preset(preset: &str) -> TableStyle {
    let mut chars = [None; COMPONENTS];

    for (slot, char) in chars.iter_mut().zip(preset.chars()) {
        *slot = (char != ' ').then_some(char);
    }

    TableStyle::new()
        .top_border(LineStyle {
            left: chars[15],
            fill: chars[2],
            junction: chars[13],
            right: chars[16],
        })
        .header_lines(ContentLineStyle {
            left: chars[0],
            junction: chars[8],
            right: chars[1],
        })
        .header_separator(LineStyle {
            left: chars[4],
            fill: chars[5],
            junction: chars[6],
            right: chars[7],
        })
        .content_lines(ContentLineStyle {
            left: chars[0],
            junction: chars[8],
            right: chars[1],
        })
        .row_separator(LineStyle {
            left: chars[11],
            fill: chars[9],
            junction: chars[10],
            right: chars[12],
        })
        .bottom_border(LineStyle {
            left: chars[17],
            fill: chars[3],
            junction: chars[14],
            right: chars[18],
        })
}

/// Replaces the control characters of `text` with U+FFFD.
///
/// A subject, a display name or a filename reaches the table as the
/// sender wrote it, so an escape sequence in it would be interpreted by
/// the terminal: rewriting the clipboard (OSC 52), hiding text or clearing
/// the screen. The JSON output keeps the original string.
pub fn printable(text: &str) -> Cow<'_, str> {
    if !text.chars().any(char::is_control) {
        return Cow::Borrowed(text);
    }

    text.chars()
        .map(|c| {
            if c.is_control() {
                char::REPLACEMENT_CHARACTER
            } else {
                c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use pimalaya_cli::table::presets;

    use super::{DEFAULT_PRESET, printable, style_from_preset};

    #[test]
    fn printable_borrows_clean_text() {
        assert!(matches!(
            printable("Re: café ☕"),
            Cow::Borrowed("Re: café ☕")
        ));
    }

    #[test]
    fn printable_replaces_control_characters() {
        // ESC and BEL (C0), DEL, and the single-byte CSI (C1).
        assert_eq!(printable("a\x1b[2Jb\x07c\x7fd\u{9b}e\tf"), "a�[2Jb�c�d�e�f");
    }

    // NOTE: equality with the v8 constant across all six line styles is
    // what proves the character-to-slot mapping.

    #[test]
    fn utf8_full_matches_upstream() {
        let preset = "││──╞═╪╡┆╌┼├┤┬┴┌┐└┘";
        assert_eq!(style_from_preset(preset), presets::UTF8_FULL);
    }

    #[test]
    fn ascii_full_matches_upstream() {
        let preset = "||--+==+|-+||++++++";
        assert_eq!(style_from_preset(preset), presets::ASCII_FULL);
    }

    #[test]
    fn ascii_markdown_matches_upstream() {
        let preset = "||  |-|||           ";
        assert_eq!(style_from_preset(preset), presets::ASCII_MARKDOWN);
    }

    #[test]
    fn utf8_no_borders_matches_upstream() {
        let preset = "     ═╪ ┆╌┼        ";
        assert_eq!(style_from_preset(preset), presets::UTF8_NO_BORDERS);
    }

    #[test]
    fn default_preset_is_utf8_full_condensed() {
        assert_eq!(
            style_from_preset(DEFAULT_PRESET),
            presets::UTF8_FULL_CONDENSED
        );
    }

    #[test]
    fn all_spaces_draws_nothing() {
        assert_eq!(style_from_preset(&" ".repeat(19)), presets::NOTHING);
    }

    #[test]
    fn missing_components_draw_nothing() {
        assert_eq!(
            style_from_preset("││──"),
            style_from_preset("││──               ")
        );
    }

    #[test]
    fn extra_characters_are_ignored() {
        assert_eq!(
            style_from_preset("││──╞═╪╡┆╌┼├┤┬┴┌┐└┘XYZ"),
            presets::UTF8_FULL
        );
    }
}
