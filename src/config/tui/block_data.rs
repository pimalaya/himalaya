use serde::Deserialize;

/// This struct stores the possible values which the user can set.
/// It's mainly a representation for the looking of each frame, like the sidebar
/// and the mail_list frame. So if you want to change the look of these frames,
/// than you're actually setting the values into this struct.
///
/// # Example
/// This is an example for the sidebar:
///
/// ```toml
/// [tui]
/// [tui.sidebar]
/// border_type = "Rounded"
/// borders = "ALL"
/// border_color = "Yellow"
/// ```
///
/// So after reading the config file, these values are stored here in this
/// struct into their appropriate attribute name.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BlockDataConfig {

    /// This stores the color of the border which can be one of [these
    /// variants](https://docs.rs/tui/0.15.0/tui/style/enum.Color.html#variants).
    pub border_color: String,

    /// Which borders of the square frame should be displayed? Default: `ALL`.
    /// For more information, take a look into their
    /// [docs](https://docs.rs/tui/0.15.0/tui/widgets/struct.Borders.html).
    pub border_type: String,

    /// So this variable stores the border type which the user wants to see. All
    /// possible options can be seen here:
    /// [here](https://docs.rs/tui/0.15.0/tui/widgets/enum.BorderType.html).
    pub borders: String,
}

impl Default for BlockDataConfig {
    fn default() -> Self {
        Self {
            border_color: String::from("Black"),
            border_type:  String::from("Rounded"),
            // Display all borders per default
            borders:      String::from("rltb"),
        }
    }
}
