use crate::domain::{Hotkey, MediaCommand, VolumeCommand};
use crate::error::AppResult;
use std::path::Path;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::SystemPlatform;

#[cfg(not(windows))]
mod fallback;

#[cfg(not(windows))]
pub use fallback::SystemPlatform;

pub trait DesktopPlatform: Send + Sync {
    fn launch_application(
        &self,
        path: &Path,
        arguments: &[String],
        working_directory: Option<&Path>,
    ) -> AppResult<()>;
    fn open_path(&self, path: &Path) -> AppResult<()>;
    fn open_url(&self, url: &str) -> AppResult<()>;
    fn send_hotkey(&self, hotkey: &Hotkey) -> AppResult<()>;
    fn type_text(&self, text: &str) -> AppResult<()>;
    fn media_command(&self, command: MediaCommand) -> AppResult<()>;
    fn volume_command(&self, command: VolumeCommand) -> AppResult<()>;
    fn play_sound(&self, path: &Path) -> AppResult<()>;
    fn close_application(&self, executable_name: &str) -> AppResult<()>;
}
