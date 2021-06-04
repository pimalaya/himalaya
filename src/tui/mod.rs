pub mod cli;
pub mod model;

pub mod modes {
    pub mod normal {
        pub mod mail_list;
        pub mod main;
        pub mod sidebar;

        pub mod widgets {
            pub mod mail_entry;
        }
    }

    pub mod widgets {
        pub mod attachments;
        pub mod header;
    }

    pub mod viewer {
        pub mod main;
        pub mod mail_content;
    }

    pub mod writer {
        pub mod main;
    }

    pub mod backend_interface;
    pub mod block_data;
    pub mod keybinding_manager;
    pub mod table_state_wrapper;
    pub mod list_state_wrapper;
    // pub mod tui_widget;
}
