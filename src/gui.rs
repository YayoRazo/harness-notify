// GuiState backs the native settings window (gui feature) and its own tests.
// Gated so it is not dead code in a --no-default-features non-test build.
#[cfg(any(feature = "gui", test))]
use crate::config::{load_config, save_config, Config};
#[cfg(any(feature = "gui", test))]
use std::path::Path;

#[cfg(any(feature = "gui", test))]
pub struct GuiState {
    pub cfg: Config,
}

#[cfg(any(feature = "gui", test))]
impl GuiState {
    pub fn load(path: &Path) -> Self {
        Self { cfg: load_config(path) }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        save_config(&self.cfg, path)
    }
}

#[cfg(feature = "gui")]
pub fn run() {
    use crate::config::default_config_path;

    let path = default_config_path();
    let mut state = GuiState::load(&path);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([360.0, 320.0]),
        ..Default::default()
    };

    let _ = eframe::run_simple_native("harness-notify settings", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("harness-notify");
            ui.checkbox(&mut state.cfg.events.done, "Notify on task complete");
            ui.checkbox(&mut state.cfg.events.attention, "Notify when input is needed");
            ui.checkbox(&mut state.cfg.events.subagent_done, "Notify on subagent finished");
            ui.checkbox(&mut state.cfg.sound.enabled, "Play sound");
            ui.checkbox(&mut state.cfg.session.include_name, "Include session name");
            ui.separator();
            ui.checkbox(&mut state.cfg.dnd.enabled, "Quiet hours");
            ui.horizontal(|ui| {
                ui.label("Start");
                ui.text_edit_singleline(&mut state.cfg.dnd.start);
                ui.label("End");
                ui.text_edit_singleline(&mut state.cfg.dnd.end);
            });
            if ui.button("Save").clicked() {
                let _ = state.save(&path);
            }
        });
    });
}

#[cfg(not(feature = "gui"))]
pub fn run() {
    eprintln!("harness-notify: this build was compiled without the `gui` feature (--no-default-features). Use `harness-notify config get/set/list` instead.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn state_loads_from_an_existing_config_file_and_saves_back() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut state = GuiState::load(&path);
        assert!(state.cfg.sound.enabled);
        state.cfg.sound.enabled = false;
        state.save(&path).unwrap();
        let reloaded = GuiState::load(&path);
        assert!(!reloaded.cfg.sound.enabled);
    }
}
