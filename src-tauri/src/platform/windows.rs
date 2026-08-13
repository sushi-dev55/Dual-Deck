use super::DesktopPlatform;
use crate::domain::{Hotkey, KeyModifier, MediaCommand, VolumeCommand};
use crate::error::{AppResult, CommandError};
use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput,
    VIRTUAL_KEY, VK_ADD, VK_APPS, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DECIMAL, VK_DELETE,
    VK_DIVIDE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_HOME, VK_INSERT, VK_LWIN, VK_MEDIA_NEXT_TRACK,
    VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK,
    VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PRIOR, VK_RETURN, VK_RIGHT,
    VK_SCROLL, VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_SUBTRACT, VK_TAB, VK_UP, VK_VOLUME_DOWN,
    VK_VOLUME_MUTE, VK_VOLUME_UP, VkKeyScanW,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR};

pub struct SystemPlatform;

impl SystemPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopPlatform for SystemPlatform {
    fn launch_application(
        &self,
        path: &Path,
        arguments: &[String],
        working_directory: Option<&Path>,
    ) -> AppResult<()> {
        ensure_regular_file(path, "applicationNotFound")?;
        let is_executable = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
        if !is_executable {
            return Err(CommandError::new(
                "invalidApplication",
                "Applications must be Windows executable files",
            ));
        }
        if arguments.len() > 64 || arguments.iter().any(|argument| argument.len() > 4096) {
            return Err(CommandError::new(
                "invalidArguments",
                "The application argument list is too large",
            ));
        }
        let mut command = Command::new(path);
        command.args(arguments);
        if let Some(directory) = working_directory {
            if !directory.is_dir() {
                return Err(CommandError::new(
                    "workingDirectoryNotFound",
                    "The working directory was not found",
                ));
            }
            command.current_dir(directory);
        }
        command.spawn()?;
        Ok(())
    }

    fn open_path(&self, path: &Path) -> AppResult<()> {
        if !path.exists() {
            return Err(CommandError::new(
                "pathNotFound",
                "The file or folder was not found",
            ));
        }
        shell_open(path.as_os_str())
    }

    fn open_url(&self, url: &str) -> AppResult<()> {
        shell_open(OsStr::new(url))
    }

    fn send_hotkey(&self, hotkey: &Hotkey) -> AppResult<()> {
        let key = virtual_key(&hotkey.key)?;
        let mut modifiers = Vec::with_capacity(hotkey.modifiers.len());
        for modifier in &hotkey.modifiers {
            let virtual_key = match modifier {
                KeyModifier::Control => VK_CONTROL,
                KeyModifier::Alt => VIRTUAL_KEY(0x12),
                KeyModifier::Shift => VK_SHIFT,
                KeyModifier::Meta => VK_LWIN,
            };
            if !modifiers.contains(&virtual_key) {
                modifiers.push(virtual_key);
            }
        }
        let mut inputs = Vec::with_capacity(modifiers.len() * 2 + 2);
        inputs.extend(
            modifiers
                .iter()
                .copied()
                .map(|key| keyboard_input(key, false)),
        );
        inputs.push(keyboard_input(key, false));
        inputs.push(keyboard_input(key, true));
        inputs.extend(
            modifiers
                .iter()
                .rev()
                .copied()
                .map(|key| keyboard_input(key, true)),
        );
        send_inputs(&inputs)
    }

    fn type_text(&self, text: &str) -> AppResult<()> {
        if text.chars().count() > 10_000 {
            return Err(CommandError::new(
                "textTooLong",
                "Typed text cannot exceed 10,000 characters",
            ));
        }
        let mut inputs = Vec::with_capacity(text.len() * 2);
        for unit in text.encode_utf16() {
            inputs.push(unicode_input(unit, false));
            inputs.push(unicode_input(unit, true));
        }
        for chunk in inputs.chunks(512) {
            send_inputs(chunk)?;
        }
        Ok(())
    }

    fn media_command(&self, command: MediaCommand) -> AppResult<()> {
        let key = match command {
            MediaCommand::PlayPause => VK_MEDIA_PLAY_PAUSE,
            MediaCommand::NextTrack => VK_MEDIA_NEXT_TRACK,
            MediaCommand::PreviousTrack => VK_MEDIA_PREV_TRACK,
            MediaCommand::Stop => VK_MEDIA_STOP,
        };
        send_inputs(&[keyboard_input(key, false), keyboard_input(key, true)])
    }

    fn volume_command(&self, command: VolumeCommand) -> AppResult<()> {
        let key = match command {
            VolumeCommand::Up => VK_VOLUME_UP,
            VolumeCommand::Down => VK_VOLUME_DOWN,
            VolumeCommand::Mute => VK_VOLUME_MUTE,
        };
        send_inputs(&[keyboard_input(key, false), keyboard_input(key, true)])
    }

    fn play_sound(&self, path: &Path) -> AppResult<()> {
        ensure_regular_file(path, "soundNotFound")?;
        let is_wave = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"));
        if !is_wave {
            return Err(CommandError::new(
                "unsupportedSoundFormat",
                "Sound actions currently support WAV files",
            ));
        }
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let played = unsafe {
            PlaySoundW(
                PCWSTR(wide.as_ptr()),
                None,
                SND_FILENAME | SND_ASYNC | SND_NODEFAULT,
            )
        };
        if played.as_bool() {
            Ok(())
        } else {
            Err(CommandError::new(
                "soundPlaybackFailed",
                "Windows could not play the selected sound",
            ))
        }
    }

    fn close_application(&self, executable_name: &str) -> AppResult<()> {
        let name = validate_executable_name(executable_name)?;
        ensure_safe_close_target(name)?;
        let status = Command::new("taskkill.exe")
            .args(["/IM", name])
            .creation_flags(0x0800_0000)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(CommandError::new(
                "applicationNotRunning",
                "The application was not running or could not be closed",
            ))
        }
    }
}

fn ensure_regular_file(path: &Path, code: &str) -> AppResult<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(CommandError::new(code, "The selected file was not found"))
    }
}

fn shell_open(target: &OsStr) -> AppResult<()> {
    let operation = HSTRING::from("open");
    let target = HSTRING::from(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            &operation,
            &target,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    let status = result.0 as isize;
    if status > 32 {
        Ok(())
    } else {
        Err(CommandError::new(
            "openFailed",
            format!("Windows could not open the selected item (code {status})"),
        ))
    }
}

fn validate_executable_name(value: &str) -> AppResult<&str> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 260
        && value.to_ascii_lowercase().ends_with(".exe")
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_. ".contains(character));
    if valid {
        Ok(value)
    } else {
        Err(CommandError::new(
            "invalidExecutableName",
            "Enter an executable file name without a path",
        ))
    }
}

fn ensure_safe_close_target(name: &str) -> AppResult<()> {
    const PROTECTED: &[&str] = &[
        "csrss.exe",
        "dual-deck.exe",
        "dwm.exe",
        "explorer.exe",
        "lsass.exe",
        "services.exe",
        "smss.exe",
        "svchost.exe",
        "wininit.exe",
        "winlogon.exe",
    ];
    let is_current_process = std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|value| value.to_owned()))
        .is_some_and(|current| current.eq_ignore_ascii_case(name));
    if is_current_process || PROTECTED.iter().any(|item| item.eq_ignore_ascii_case(name)) {
        Err(CommandError::new(
            "protectedApplication",
            "Dual Deck cannot close this Windows process",
        ))
    } else {
        Ok(())
    }
}

fn keyboard_input(key: VIRTUAL_KEY, released: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: if released {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                ..Default::default()
            },
        },
    }
}

fn unicode_input(unit: u16, released: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wScan: unit,
                dwFlags: if released {
                    KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                } else {
                    KEYEVENTF_UNICODE
                },
                ..Default::default()
            },
        },
    }
}

fn send_inputs(inputs: &[INPUT]) -> AppResult<()> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(CommandError::new(
            "inputInjectionFailed",
            "Windows did not accept the keyboard input",
        ))
    }
}

fn virtual_key(value: &str) -> AppResult<VIRTUAL_KEY> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.len() == 1 {
        let byte = normalized.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Ok(VIRTUAL_KEY(byte as u16));
        }
    }
    let mut units = value.trim().encode_utf16();
    if let (Some(unit), None) = (units.next(), units.next()) {
        let mapped = unsafe { VkKeyScanW(unit) };
        if mapped != -1 {
            return Ok(VIRTUAL_KEY((mapped as u16) & 0x00ff));
        }
    }
    if let Some(number) = normalized
        .strip_prefix('F')
        .and_then(|number| number.parse::<u16>().ok())
        .filter(|number| (1..=24).contains(number))
    {
        return Ok(VIRTUAL_KEY(VK_F1.0 + number - 1));
    }
    let key = match normalized.as_str() {
        "BACKSPACE" => VK_BACK,
        "TAB" => VK_TAB,
        "ENTER" | "RETURN" => VK_RETURN,
        "ESC" | "ESCAPE" => VK_ESCAPE,
        "SPACE" => VK_SPACE,
        "PAGEUP" => VK_PRIOR,
        "PAGEDOWN" => VK_NEXT,
        "END" => VK_END,
        "HOME" => VK_HOME,
        "LEFT" => VIRTUAL_KEY(0x25),
        "UP" => VK_UP,
        "RIGHT" => VK_RIGHT,
        "DOWN" => VK_DOWN,
        "PRINTSCREEN" => VK_SNAPSHOT,
        "INSERT" => VK_INSERT,
        "DELETE" => VK_DELETE,
        "CAPSLOCK" => VK_CAPITAL,
        "NUMLOCK" => VK_NUMLOCK,
        "SCROLLLOCK" => VK_SCROLL,
        "APPS" | "MENU" => VK_APPS,
        "PLUS" => VK_OEM_PLUS,
        "MINUS" => VK_OEM_MINUS,
        "COMMA" => VK_OEM_COMMA,
        "PERIOD" => VK_OEM_PERIOD,
        "NUMPAD_ADD" => VK_ADD,
        "NUMPAD_SUBTRACT" => VK_SUBTRACT,
        "NUMPAD_MULTIPLY" => VK_MULTIPLY,
        "NUMPAD_DIVIDE" => VK_DIVIDE,
        "NUMPAD_DECIMAL" => VK_DECIMAL,
        _ => {
            return Err(CommandError::new(
                "invalidHotkey",
                format!("Unsupported hotkey: {}", value.trim()),
            ));
        }
    };
    Ok(key)
}

use std::os::windows::ffi::OsStrExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_keys() {
        assert_eq!(virtual_key("A").expect("A").0, 65);
        assert_eq!(virtual_key("f24").expect("F24").0, VK_F1.0 + 23);
        assert!(virtual_key(";").is_ok());
        assert!(virtual_key("not-a-key").is_err());
    }

    #[test]
    fn rejects_paths_as_process_names() {
        assert!(validate_executable_name("obs64.exe").is_ok());
        assert!(validate_executable_name("C:\\OBS\\obs64.exe").is_err());
        assert!(validate_executable_name("obs64").is_err());
        assert!(ensure_safe_close_target("explorer.exe").is_err());
    }
}
