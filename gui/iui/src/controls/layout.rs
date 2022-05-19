use super::Control;
use error::UIError;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_int;
use ui::UI;
use ui_sys::{self, uiAlign, uiAt, uiBox, uiControl, uiGrid, uiGroup, uiSeparator, uiTab};

/// Defines the ways in which the children of boxes can be layed out.
pub enum LayoutStrategy {
    /// Make the control the minimum possible size to contain its content
    Compact,
    /// Make the control expand to its maximum size
    Stretchy,
}

define_control! {
    /// Lays out its children vertically.
    rust_type: VerticalBox,
    sys_type: uiBox
}

define_control! {
    /// Lays out its children horizontally.
    rust_type: HorizontalBox,
    sys_type: uiBox
}

impl VerticalBox {
    /// Create a new vertical box layout.
    pub fn new(_ctx: &UI) -> VerticalBox {
        VerticalBox {
            uiBox: unsafe { ui_sys::uiNewVerticalBox() },
        }
    }
}

impl HorizontalBox {
    /// Create a new horizontal box layout.
    pub fn new(_ctx: &UI) -> HorizontalBox {
        HorizontalBox {
            uiBox: unsafe { ui_sys::uiNewHorizontalBox() },
        }
    }
}

fn append<T: Into<Control>>(b: *mut uiBox, ctx: &UI, child: T, strategy: LayoutStrategy) {
    let stretchy = match strategy {
        LayoutStrategy::Compact => false,
        LayoutStrategy::Stretchy => true,
    };
    let control = child.into();
    unsafe {
        assert!(ctx.parent_of(control.clone()).is_none());
        ui_sys::uiBoxAppend(b, control.ui_control, stretchy as c_int)
    }
}

fn padded(b: *mut uiBox, _ctx: &UI) -> bool {
    unsafe { ui_sys::uiBoxPadded(b) != 0 }
}

fn set_padded(b: *mut uiBox, padded: bool, _ctx: &UI) {
    unsafe { ui_sys::uiBoxSetPadded(b, padded as c_int) }
}

impl VerticalBox {
    /// Add a control to the end of the box, sized by the given layout strategy.
    pub fn append<T: Into<Control>>(&mut self, _ctx: &UI, child: T, strategy: LayoutStrategy) {
        append(self.uiBox, _ctx, child, strategy)
    }

    /// Determine whenther the box provides padding around its children.
    pub fn padded(&self, _ctx: &UI) -> bool {
        padded(self.uiBox, _ctx)
    }

    /// Set whether or not the box should provide padding around its children.
    pub fn set_padded(&mut self, _ctx: &UI, padded: bool) {
        set_padded(self.uiBox, padded, _ctx)
    }
}

impl HorizontalBox {
    /// Add a control to the end of the box, sized by the given layout strategy.
    pub fn append<T: Into<Control>>(&mut self, _ctx: &UI, child: T, strategy: LayoutStrategy) {
        append(self.uiBox, _ctx, child, strategy)
    }

    /// Determine whenther the box provides padding around its children.
    pub fn padded(&self, _ctx: &UI) -> bool {
        padded(self.uiBox, _ctx)
    }

    /// Set whether or not the box should provide padding around its children.
    pub fn set_padded(&mut self, _ctx: &UI, padded: bool) {
        set_padded(self.uiBox, padded, _ctx)
    }
}

define_control! {
    /// Group of tabs, each of which shows a different sub-control.
    rust_type: TabGroup,
    sys_type: uiTab
}

define_control! {
    /// Collects controls together, with (optionally) a margin and/or title.
    rust_type: Group,
    sys_type: uiGroup
}

impl Group {
    /// Create a new group with the given title.
    pub fn new(_ctx: &UI, title: &str) -> Group {
        let mut group = unsafe {
            let c_string = CString::new(title.as_bytes().to_vec()).unwrap();
            Group::from_raw(ui_sys::uiNewGroup(c_string.as_ptr()))
        };
        group.set_margined(_ctx, true);
        group
    }

    /// Get a copy of the current group title.
    pub fn title(&self, _ctx: &UI) -> String {
        unsafe {
            CStr::from_ptr(ui_sys::uiGroupTitle(self.uiGroup))
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Get a reference to the existing group title.
    pub fn title_ref(&self, _ctx: &UI) -> &CStr {
        unsafe { CStr::from_ptr(ui_sys::uiGroupTitle(self.uiGroup)) }
    }

    // Set the group's title.
    pub fn set_title(&mut self, _ctx: &UI, title: &str) {
        unsafe {
            let c_string = CString::new(title.as_bytes().to_vec()).unwrap();
            ui_sys::uiGroupSetTitle(self.uiGroup, c_string.as_ptr())
        }
    }

    // Set the group's child widget.
    pub fn set_child<T: Into<Control>>(&mut self, _ctx: &UI, child: T) {
        unsafe { ui_sys::uiGroupSetChild(self.uiGroup, child.into().ui_control) }
    }

    // Check whether or not the group draws a margin.
    pub fn margined(&self, _ctx: &UI) -> bool {
        unsafe { ui_sys::uiGroupMargined(self.uiGroup) != 0 }
    }

    // Set whether or not the group draws a margin.
    pub fn set_margined(&mut self, _ctx: &UI, margined: bool) {
        unsafe { ui_sys::uiGroupSetMargined(self.uiGroup, margined as c_int) }
    }
}

impl TabGroup {
    /// Create a new, empty group of tabs.
    pub fn new(_ctx: &UI) -> TabGroup {
        unsafe { TabGroup::from_raw(ui_sys::uiNewTab()) }
    }

    /// Add the given control as a new tab in the tab group with the given name.
    ///
    /// Returns the number of tabs in the group after adding the new tab.
    pub fn append<T: Into<Control>>(&mut self, _ctx: &UI, name: &str, control: T) -> i32 {
        let control = control.into();
        unsafe {
            let c_string = CString::new(name.as_bytes().to_vec()).unwrap();
            ui_sys::uiTabAppend(self.uiTab, c_string.as_ptr(), control.ui_control);
            ui_sys::uiTabNumPages(self.uiTab) as i32
        }
    }

    /// Add the given control before the given index in the tab group, as a new tab with a given name.
    ///
    /// Returns the number of tabs in the group after adding the new tab.
    pub fn insert_at<T: Into<Control>>(
        &mut self,
        _ctx: &UI,
        name: &str,
        before: i32,
        control: T,
    ) -> i32 {
        unsafe {
            let c_string = CString::new(name.as_bytes().to_vec()).unwrap();
            ui_sys::uiTabInsertAt(
                self.uiTab,
                c_string.as_ptr(),
                before,
                control.into().ui_control,
            );
            ui_sys::uiTabNumPages(self.uiTab) as i32
        }
    }

    /// Remove the control at the given index in the tab group.
    ///
    /// Returns the number of tabs in the group after removing the tab, or an error if that index was out of bounds.
    ///
    /// NOTE: This will leak the deleted control! We have no way of actually getting it
    /// to decrement its reference count per `libui`'s UI as of today, unless we maintain a
    /// separate list of children ourselves…
    pub fn delete(&mut self, _ctx: &UI, index: i32) -> Result<i32, UIError> {
        let n = unsafe { ui_sys::uiTabNumPages(self.uiTab) as i32 };
        if index < n {
            unsafe { ui_sys::uiTabDelete(self.uiTab, index) };
            Ok(n)
        } else {
            Err(UIError::TabGroupIndexOutOfBounds { index, n })
        }
    }

    /// Determine whether or not the tab group provides margins around its children.
    pub fn margined(&self, _ctx: &UI, page: i32) -> bool {
        unsafe { ui_sys::uiTabMargined(self.uiTab, page) != 0 }
    }

    /// Set whether or not the tab group provides margins around its children.
    pub fn set_margined(&mut self, _ctx: &UI, page: i32, margined: bool) {
        unsafe { ui_sys::uiTabSetMargined(self.uiTab, page, margined as c_int) }
    }
}

define_control! {
    /// Horizontal line, to seperate things visually.
    rust_type: HorizontalSeparator,
    sys_type: uiSeparator
}

impl HorizontalSeparator {
    pub fn new(_ctx: &UI) -> Self {
        unsafe { HorizontalSeparator::from_raw(ui_sys::uiNewHorizontalSeparator()) }
    }
}

define_control! {
    /// Seperates components with empty space.
    rust_type: Spacer,
    sys_type: uiBox
}

impl Spacer {
    pub fn new(_ctx: &UI) -> Self {
        unsafe { Spacer::from_raw(ui_sys::uiNewHorizontalBox()) }
    }
}

/// Informs a `LayoutGrid` about how a control should use available space
/// in one or both dimensions.
pub enum GridExpand {
    /// This control should not use extra space
    Neither,
    /// This control should use extra space horizontally
    Horizontal,
    /// This control should use extra space vertically
    Vertical,
    /// This control should use all available space in both dimensions
    Both,
}

/// Informs a `LayoutGrid` how to align a control.
#[derive(Clone, Copy, PartialEq)]
pub enum GridAlignment {
    /// Expand to use all available space.
    Fill,
    /// Collapse toward the start of the available space.
    Start,
    /// Collapse equally on both sides of the available space.
    Center,
    /// Collapse toward the end of the available space.
    End,
}

impl GridAlignment {
    fn into_ui_align(self) -> uiAlign {
        use self::GridAlignment::*;
        return match self {
            Fill => ui_sys::uiAlignFill,
            Start => ui_sys::uiAlignStart,
            Center => ui_sys::uiAlignCenter,
            End => ui_sys::uiAlignEnd,
        } as uiAlign;
    }
}

/// Informs a `LayoutGrid` as to position a control.
#[derive(Clone, Copy, PartialEq)]
pub enum GridInsertionStrategy {
    /// Place control to left of existing control, align tops
    Leading,
    /// Place control above existing control, align left edges
    Top,
    /// Place control to right of existing control, align tops
    Trailing,
    /// Place control below existing control, align left edges
    Bottom,
}

impl GridInsertionStrategy {
    fn into_ui_at(self) -> uiAt {
        use self::GridInsertionStrategy::*;
        return match self {
            Leading => ui_sys::uiAtLeading,
            Top => ui_sys::uiAtTop,
            Trailing => ui_sys::uiAtTrailing,
            Bottom => ui_sys::uiAtBottom,
        } as uiAlign;
    }
}

define_control! {
    /// Lays out its children in a grid according to insertion instructions.
    rust_type: LayoutGrid,
    sys_type: uiGrid
}

impl LayoutGrid {
    /// Creates a new `LayoutGrid`.
    pub fn new(_ctx: &UI) -> Self {
        unsafe { LayoutGrid::from_raw(ui_sys::uiNewGrid()) }
    }

    /// Returns `true` if the `LayoutGrid` is padded and `false` if not.
    pub fn padded(&self, _ctx: &UI) -> bool {
        if unsafe { ui_sys::uiGridPadded(self.uiGrid) } == 0 {
            true
        } else {
            false
        }
    }

    /// Sets the padding state of the `LayoutGrid`
    pub fn set_padded(&mut self, _ctx: &UI, padded: bool) {
        let v = if padded { 1 } else { 0 };

        unsafe {
            ui_sys::uiGridSetPadded(self.uiGrid, v);
        }
    }

    /// Adds a control to the `LayoutGrid`.
    pub fn append<T: Into<Control>>(
        &mut self,
        _ctx: &UI,
        control: T,
        left: i32,
        height: i32,
        xspan: i32,
        yspan: i32,
        expand: GridExpand,
        halign: GridAlignment,
        valign: GridAlignment,
    ) {
        let (hexpand, vexpand) = match expand {
            GridExpand::Neither => (0, 0),
            GridExpand::Horizontal => (1, 0),
            GridExpand::Vertical => (0, 1),
            GridExpand::Both => (1, 1),
        };
        unsafe {
            ui_sys::uiGridAppend(
                self.uiGrid,
                control.into().ui_control,
                left,
                height,
                xspan,
                yspan,
                hexpand,
                halign.into_ui_align(),
                vexpand,
                valign.into_ui_align(),
            );
        }
    }

    /// Inserts a control in to the `LayoutGrid` relative to an existing control.
    pub fn insert_at<T: Into<Control>, U: Into<Control>>(
        &mut self,
        _ctx: &UI,
        control: T,
        existing: U,
        at: GridInsertionStrategy,
        xspan: i32,
        yspan: i32,
        expand: GridExpand,
        halign: GridAlignment,
        valign: GridAlignment,
    ) {
        let (hexpand, vexpand) = match expand {
            GridExpand::Neither => (0, 0),
            GridExpand::Horizontal => (1, 0),
            GridExpand::Vertical => (0, 1),
            GridExpand::Both => (1, 1),
        };
        unsafe {
            ui_sys::uiGridInsertAt(
                self.uiGrid,
                control.into().ui_control,
                existing.into().ui_control,
                at.into_ui_at(),
                xspan,
                yspan,
                hexpand,
                halign.into_ui_align(),
                vexpand,
                valign.into_ui_align(),
            );
        }
    }
}
