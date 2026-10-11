use std::path::{Path, PathBuf};

use confy::ConfyError;
use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::{ffmpeg::RenderSettings, osd::OsdOptions, util::AppUpdate, NAMESPACE};

#[derive(Debug, Deserialize, Serialize, Derivative)]
#[derivative(Default)]
pub struct AppConfig {
    pub osd_options: OsdOptions,
    pub render_options: RenderSettings,
    pub app_update: AppUpdate,
    pub font_path: String,
}

const CONFIG_NAME: &str = "saved_settings";

/// A file with this name next to the executable switches on portable mode.
const PORTABLE_MARKER: &str = "portable.txt";
const PORTABLE_DATA_DIR: &str = "data";

/// Folder for the settings and logs in portable mode. In portable mode, which is switched on by a file
/// `portable.txt` next to the executable, they are stored in the folder `data` next to the executable
/// instead of the folders of the user, so the program can be moved around as a whole.
pub fn portable_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    portable_data_dir_for(exe.parent()?)
}

fn portable_data_dir_for(exe_dir: &Path) -> Option<PathBuf> {
    exe_dir
        .join(PORTABLE_MARKER)
        .is_file()
        .then(|| exe_dir.join(PORTABLE_DATA_DIR))
}

fn config_file_path() -> Option<PathBuf> {
    portable_data_dir().map(|dir| dir.join(format!("{CONFIG_NAME}.ron")))
}

impl AppConfig {
    #[tracing::instrument(ret)]
    pub fn load_or_create() -> Self {
        Self::load_or_create_at(config_file_path().as_deref())
    }

    #[tracing::instrument]
    pub fn save(&self) {
        self.save_at(config_file_path().as_deref())
    }

    /// Uses the config file at `path` or the config file in the folder of the user if there is no path.
    fn load_or_create_at(path: Option<&Path>) -> Self {
        let config: Result<Self, _> = match path {
            Some(path) => confy::load_path(path),
            None => confy::load(NAMESPACE, CONFIG_NAME),
        };
        if let Err(ConfyError::BadRonData(_)) = config {
            tracing::warn!("Invalid config found, resetting to default");
            let default_config = AppConfig::default();
            tracing::debug!("Default config: {:?}", default_config);
            default_config.save_at(path);
            default_config
        } else {
            config
                .map_err(|e| tracing::error!("Failed to load or create new config, caused by {e}"))
                .unwrap()
        }
    }

    fn save_at(&self, path: Option<&Path>) {
        match path {
            Some(path) => confy::store_path(path, self),
            None => confy::store(NAMESPACE, CONFIG_NAME, self),
        }
        .map_err(|e| tracing::error!("Failed to save config file, {}", e))
        .ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("abwrt_config_test_{}_{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn portable_mode_needs_the_marker_file() {
        let dir = temp_dir("marker");
        assert_eq!(portable_data_dir_for(&dir), None);

        std::fs::write(dir.join(PORTABLE_MARKER), "").unwrap();
        assert_eq!(portable_data_dir_for(&dir), Some(dir.join("data")));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn config_is_saved_and_loaded_from_the_given_path() {
        let dir = temp_dir("roundtrip");
        // The data folder does not exist yet and is created when saving
        let path = dir.join("data").join("saved_settings.ron");

        // A first start creates the default config
        assert_eq!(AppConfig::load_or_create_at(Some(&path)).font_path, "");
        assert!(path.is_file());

        let config = AppConfig {
            font_path: "my_font.png".into(),
            ..Default::default()
        };
        config.save_at(Some(&path));
        assert_eq!(AppConfig::load_or_create_at(Some(&path)).font_path, "my_font.png");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn invalid_config_is_reset() {
        let dir = temp_dir("invalid");
        let path = dir.join("saved_settings.ron");
        std::fs::write(&path, "this is not a config").unwrap();

        assert_eq!(AppConfig::load_or_create_at(Some(&path)).font_path, "");
        // The file was replaced by a valid default config
        assert_eq!(AppConfig::load_or_create_at(Some(&path)).font_path, "");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
