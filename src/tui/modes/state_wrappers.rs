//! This file include wrappers for the [ListState] and the [TableState] struct
//! which creates a higher abstract layer for scrolling in those two widgets.
//!
//! # Reason
//! So the most widgets, like the `mail_list` and the `sidebar`, are using the
//! [TableState] widget in order to be able to scroll through their entrys. Both
//! needed the same functions with the same function-body. So instead of copy +
//! pasting the "big" functions all the time, these two wrapper-structs were
//! created.
//!
//! So whenever you need another scrollable (table)/(list) you can use their
//! appropriate wrapper.
//!
//! Each struct has a little API to interact with the actual states provided by
//! [tui-rs]. Just take a look into the description of the methods.
//!
//! # Example
//! ```rust
//! # use himalaya::tui::model::state_wrapper::{
//! #     TableStateWrapper, TableWrapperFuncs,
//! # };
//! # use himalaya::tui::modes::block_data::BlockData;
//! pub struct TuiWidget {
//!     // This variable holds the information of our frame, how it should look
//!     // like. For example if the border should be round or which color it
//!     // has.
//!     pub block_data: BlockData,
//!
//!     // Our table will just include some strings to display.
//!     content: Vec<String>,
//!
//!     // So this widget includes a table => Use the wrapper for the table
//!     pub state: TableStateWrapper,
//! }
//!
//! impl TuiWidget {
//!     // Now the functions for our current widget like the constructor and so
//!     // on
//! }
//! ```
//! That's it!
//!
//! [TableStateWrapper]: struct.TableStateWrapper.html
//! [ListStateWrapper]: struct.ListStateWrapper.html
//! [tui-rs]: <https://github.com/fdehau/tui-rs>
//! [ListState]: <https://docs.rs/tui/0.15.0/tui/widgets/struct.ListState.html>
//! [TableState]: <https://docs.rs/tui/0.15.0/tui/widgets/struct.TableState.html>

use tui_rs::widgets::{ListState, TableState};

// =====================
// ListStateWrapper
// =====================
/// The wrapper for [ListState].
///
/// The [TableStateWrapper] and the [ListStateWrapper] are almost equal, so
/// everything which is explained here is **valid for both**.
///
/// # Usage
/// This is used, if you want to have scrollable [list widget].
///
/// # Example
/// Take a look into the example of the [state_wrappers].
///
/// [TableStateWrapper]: struct@TableStateWrapper
/// [ListStateWrapper]: struct@ListStateWrapper
/// [ListState]: https://docs.rs/tui/0.15.0/tui/widgets/struct.ListState.html 
/// [list widget]: https://docs.rs/tui/0.15.0/tui/widgets/struct.List.html
/// [state_wrappers]: ./index.html#example
pub struct ListStateWrapper {

    /// This variable holds the actual [state] of the list which is provided of
    /// tui-rs. So we're actually interacting with this variable, in all
    /// functions of this struct.
    ///
    /// [state]: <https://docs.rs/tui/0.15.0/tui/widgets/struct.ListState.html>
    pub state:   ListState,
    list_length: usize,
}

impl ListStateWrapper {
    
    /// As all constructors do: Create a new instance of the struct. Our
    /// [ListStateWrapper.state] is gonna be the default value of `ListState`.
    /// You can optionally add the size of the list to the constructor or give
    /// `None` which will set the length of the list to 0. You'll need to update
    /// the length by the [`set_length()`] function later than!
    ///
    /// [ListStateWrapper.state]: struct@ListStateWrapper
    /// [set_length()]: struct.ListStateWrapper.html#method.set_length
    pub fn new(list_length: Option<usize>) -> Self {

        let list_length = match list_length {
            Some(size) => size,
            None => 0,
        };

        let mut state = ListState::default();
        state.select(Some(0));

        Self {
            state,
            list_length,
        }
    }

    /// This is for the relative movement of the cursor. If you enter `5` for
    /// example, than your cursor will move 5 entrys to the **down** since our
    /// list which should be displayed, is starting from the top and goes down
    /// to the bottom!
    /// 
    /// # Note
    /// This function makes sure, that the index doesn't go below `0` and
    /// greater than the length of our list. You can set the length of the list
    /// by using the [`set_length`] function.
    ///
    /// [`set_length`]: struct.ListStateWrapper.html#method.set_length
    pub fn move_cursor(&mut self, offset: i32) {
        let new_selection = match self.state.selected() {

            // Look first, if we even have a cursor
            Some(old_selection) => {

                // make sure that the new index doesn't go below 0 when
                // subtracting the offset on it
                let mut selection = if offset < 0 {
                    old_selection.saturating_sub(offset.abs() as usize)
                } else {
                    old_selection.saturating_add(offset as usize)
                };

                // make sure that the new index doesn't goes beyond the greatest
                // possible index!
                if selection > self.list_length - 1 {
                    selection = self.list_length - 1;
                }

                selection
            },
            // If something goes wrong: Move the cursor to the beginning of the
            // selections.
            None => 0,
        };

        // refresh our selection/cursor
        self.state.select(Some(new_selection));
    }

    /// It's almost the same as [`move_cursor`]. The difference is, that you
    /// move the cursor/selection to an absolute index and not relatively from
    /// the current position. You can set `index` to [None] to get to the last
    /// entry.
    ///
    /// # Note
    /// This function makes sure that the index doesn't exceed the length of the
    /// list. If it does, the cursor will be placed at the end of the entry.
    ///
    /// [`move_cursor`]: struct.ListStateWrapper.html#method.move_cursor 
    /// [None]: <https://doc.rust-lang.org/std/option/enum.Option.html#variant.None>
    pub fn set_cursor(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            if index >= self.list_length {
                self.state.select(Some(self.list_length - 1));
            } else {
                self.state.select(Some(index));
            }
        } else {
            self.state.select(Some(self.list_length - 1));
        }
    }

    /// As the name says: This will remove the cursor from the list so it's not
    /// displayed anymore.
    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    /// Sets the cursor to the beginning of the entrys.
    pub fn reset(&mut self) {
        self.state.select(Some(0));
    }

    /// If the size of your list changed, call this function. This will refresh
    /// the max-length value for our wrapper, which is used for example in the 
    /// [`move_cursor`] function to make sure, that the given index doesn't
    /// exceed the length of the list.
    ///
    /// [`move_cursor`]: struct.ListStateWrapper.html#method.move_cursor 
    pub fn set_length(&mut self, length: usize) {
        self.list_length = length;
    }

    /// This will return the index of the current selected entry of your cursor
    /// in order to know where the user is currently.
    pub fn get_selected_index(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    /// Get the low-level state of the ListState. This is mainly used, if you
    /// want to render the widget. 
    ///
    /// # Example
    /// ```rust
    /// let tui_widget = TuiWidget::new();
    ///
    /// // draw our widget
    /// frame.render_stateful_widget(
    ///     // get the widget which should be displayed
    ///     tui_widget.widget(),
    ///
    ///     // get the "frame"/"rect" where the widget has to be placed
    ///     Rect::new(0, 0, 100, 100),
    ///
    ///     // (THIS FUNCTION) get the state which is gonna be adjusted
    ///     // according to the cursor
    ///     tui_widget.get_state(),
    /// );
    /// ```
    pub fn get_state(&mut self) -> &mut ListState {
        &mut self.state
    }
}

// ======================
// TableStateWrapper
// ======================
/// The wrapper for [TableState].
///
/// As explained in the [state_wrappers] section, this struct is **almost the
/// same** as [ListState]. So take a look into [ListState] to understand the
/// functions and struct fields.
///
/// [TableState]: <https://docs.rs/tui/0.15.0/tui/widgets/struct.TableState.html>
/// [ListState]: struct@ListStateWrapper
/// [state_wrappers]: <./index.html>
pub struct TableStateWrapper {

    /// This variable holds the actual [state] of the table which is provided of
    /// tui-rs. So we're actually interacting with this variable, in all
    /// functions of this struct.
    ///
    /// [state]: <https://docs.rs/tui/0.15.0/tui/widgets/struct.TableState.html> 
    pub state:    TableState,
    table_length: usize,
}

// If you're looking for the comments of the code, take a look into the
// ListStateWrapper. Each comment is suitable to this struct as well.
impl TableStateWrapper {
    pub fn new(table_length: Option<usize>) -> Self {

        let table_length = match table_length {
            Some(size) => size,
            None => 0,
        };

        let mut state = TableState::default();
        state.select(Some(0));

        Self {
            state,
            table_length,
        }
    }

    pub fn move_cursor(&mut self, offset: i32) {
        let new_selection = match self.state.selected() {
            Some(old_selection) => {
                let mut selection = if offset < 0 {
                    old_selection.saturating_sub(offset.abs() as usize)
                } else {
                    old_selection.saturating_add(offset as usize)
                };

                if selection > self.table_length - 1 {
                    selection = self.table_length - 1;
                }

                selection
            },
            None => 0,
        };

        self.state.select(Some(new_selection));
    }

    pub fn set_cursor(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            if index >= self.table_length {
                self.state.select(Some(self.table_length - 1));
            } else {
                self.state.select(Some(index));
            }
        } else {
            self.state.select(Some(self.table_length - 1));
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn reset(&mut self) {
        self.state = TableState::default();
        self.table_length = 0;
    }

    pub fn set_length(&mut self, length: usize) {
        self.table_length = length;
    }

    pub fn get_selected_index(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    pub fn get_state(&mut self) -> &mut TableState {
        &mut self.state
    }
}
