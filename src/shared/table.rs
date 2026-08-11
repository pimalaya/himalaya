//! Shared `comfy_table` preset mapper.
//!
//! `comfy_table` v8 dropped the positional preset string in favour of
//! the typed [`TableStyle`] builder. The `table.preset` config option
//! keeps accepting the v7 string so existing configs stay valid; this
//! module maps one onto the other.

use comfy_table::{ContentLineStyle, LineStyle, TableStyle};

/// Default preset, equivalent to `comfy_table` v7's
/// `presets::UTF8_FULL_CONDENSED`: full UTF8 borders, no divider
/// between rows.
pub const DEFAULT_PRESET: &str = "││──╞═╪╡┆    ┬┴┌┐└┘";

/// Number of table components a preset string can style.
const COMPONENTS: usize = 19;

/// Maps a `comfy_table` v7 positional preset string onto a
/// [`TableStyle`].
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
/// A space means "don't draw this component", and so does a component
/// left out of a short string, both matching v7 where an unset
/// component rendered blank. Characters past the 19th are ignored.
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

#[cfg(test)]
mod tests {
    use comfy_table::presets;

    use super::{DEFAULT_PRESET, style_from_preset};

    // The v7 preset strings, mapped against the v8 constants they were
    // replaced by. Equality across all six line styles proves the
    // character-to-builder-slot mapping.

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
        // A short string leaves the remaining components unset, exactly
        // as an explicit run of spaces would.
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
