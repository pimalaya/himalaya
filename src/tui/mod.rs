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
    pub mod state_wrappers;
}

pub mod tabs {
    pub mod account_tab;
    pub mod main;
    pub mod data {
        pub mod normal_data;
        pub mod viewer_data;
        pub mod writer_data;

        pub mod shared_widgets {
            pub mod header;
        }
    }
}
