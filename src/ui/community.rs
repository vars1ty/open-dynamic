use crate::{
    mod_cores::base_core::BaseCore,
    utils::{
        cryptutils::CryptUtils,
        eguiutils::{ContentFrameData, ImGuiUtils},
    },
};
use hudhook::imgui::{self};
use parking_lot::{Mutex, RwLock};
use std::{cell::RefCell, sync::Arc};

/// Community-published item data.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CommunityItem {
    /// Item name.
    pub name: String,

    /// Item summary.
    pub summary: String,

    /// Rune code.
    pub code: String,
}

/// Community Window for accessing community-published Rune scripts.
pub struct CommunityWindow {
    /// Base Core which we use to access the script virtual machine.
    base_core: Arc<RwLock<BaseCore>>,

    /// Community item data.
    community_scripts: Arc<Mutex<Vec<CommunityItem>>>,

    content_frame_data: RefCell<ContentFrameData>,

    /// Center group size.
    group_size: [f32; 2],
}

thread_safe_structs!(CommunityWindow);

impl CommunityWindow {
    /// Initializes the window.
    pub fn init(base_core: Arc<RwLock<BaseCore>>) -> Self {
        let base_core_clone = Arc::clone(&base_core);
        let community_scripts = Arc::new(Mutex::new(Vec::new()));
        let community_scripts_clone = Arc::clone(&community_scripts);

        // Request content in a new task, since we don't want to block the main thread.
        std::thread::spawn(move || {
            log!("Requesting community content...");
            *community_scripts_clone.lock() = base_core_clone
                .read()
                .get_crosscom()
                .read()
                .get_community_content();
            log!("Community content received!");
        });

        Self {
            base_core,
            community_scripts,
            content_frame_data: RefCell::default(),
            group_size: [0.0, 0.0],
        }
    }

    /// Draws the window and its content.
    pub fn draw(&mut self, ui: &imgui::Ui) {
        self.group_size = ImGuiUtils::draw_centered_widgets(
            ui,
            ui.window_size(),
            &self.group_size,
            Some(30.0),
            || {
                let Some(font) = ImGuiUtils::activate_font(ui, 1) else {
                    log!("[ERROR] Failed activating a dynamic pre-installed font at index 1!");
                    return;
                };

                label!(ui, "󰡉 Community Submissions");
                font.pop();
            },
        );

        ui.columns(3, zencstr!("ViewColumn"), false);
        let Some(community_scripts) = self.community_scripts.try_lock() else {
            return;
        };

        for item_data in &*community_scripts {
            ImGuiUtils::draw_content_frame(
                ui,
                &item_data.name,
                &mut self.content_frame_data.borrow_mut(),
                || {
                    self.draw_inner_frame_content(ui, item_data);
                },
            );
        }
    }

    /// Draws the inner content of a community item.
    fn draw_inner_frame_content(&self, ui: &imgui::Ui, item_data: &CommunityItem) {
        label!(ui, &item_data.summary);
        if button!(ui, zencstr!("󱐋 Execute ", &item_data.name)) {
            let Some(base_core_reader) = self.base_core.try_read() else {
                return;
            };

            base_core_reader.get_script_core().execute(
                CryptUtils::decrypt(&item_data.code),
                Arc::clone(&self.base_core),
                false,
                false,
            );
        }
    }
}
