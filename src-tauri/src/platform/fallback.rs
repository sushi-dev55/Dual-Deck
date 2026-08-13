use super::DesktopPlatform;
use crate::domain::{Hotkey, MediaCommand, VolumeCommand};
use crate::error::{AppResult, CommandError};
use std::path::Path;
use std::process::Command;

pub struct SystemPlatform;

impl SystemPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl DesktopPlatform for SystemPlatform {
    fn launch_application(
        &self,
        path: &Path,
        arguments: &[String],
        working_directory: Option<&Path>,
    ) -> AppResult<()> {
        let mut command = Command::new(path);
        command.args(arguments);
        if let Some(directory) = working_directory {
            command.current_dir(directory);
        }
        command.spawn()?;
        Ok(())
    }

    fn open_path(&self, path: &Path) -> AppResult<()> {
        Command::new("xdg-open").arg(path).spawn()?;
        Ok(())
    }

    fn open_url(&self, url: &str) -> AppResult<()> {
        Command::new("xdg-open").arg(url).spawn()?;
        Ok(())
    }

    fn send_hotkey(&self, _hotkey: &Hotkey) -> AppResult<()> {
        unsupported()
    }

    fn type_text(&self, _text: &str) -> AppResult<()> {
        unsupported()
    }

    fn media_command(&self, _command: MediaCommand) -> AppResult<()> {
        unsupported()
    }

    fn volume_command(&self, _command: VolumeCommand) -> AppResult<()> {
        unsupported()
    }

    fn play_sound(&self, _path: &Path) -> AppResult<()> {
        unsupported()
    }

    fn close_application(&self, _executable_name: &str) -> AppResult<()> {
        unsupported()
    }
}

fn unsupported() -> AppResult<()> {
    Err(CommandError::new(
        "unsupportedPlatform",
        "This action is not available on the current platform",
    ))
}
