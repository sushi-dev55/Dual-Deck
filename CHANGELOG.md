# Changelog

All notable changes to Dual Deck will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases use
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

No versioned public release has been published yet. The entries below describe the current default
branch and will be moved into a dated release section when a release is created.

## Unreleased

### Added

- Tauri 2 and Svelte 5 Windows application foundation
- Local SQLite profiles, mappings, settings, migrations, and consistency validation
- SDL 3 DualSense discovery, normalized input events, and single-controller policy
- Notification-area lifecycle, profile switching, and mapping pause controls
- Single-button Press, Release, Long press, Double press, and Hold trigger editing
- Typed Windows actions for launching, keyboard input, media, volume, WAV playback, webhooks, and
  profile switching
- Automated frontend and Rust verification on Windows
- Architecture, hardware verification, contribution, code-of-conduct, security, and release
  documentation
- Front-facing photographed DualSense map with calibrated interactive controls and attributed media
- Motion system for controller depth, hardware presses, drag feedback, navigation, and view changes

### Release boundaries

- Multi-actions, profile import/export and backup, native OBS control, and automatic updates are not
  exposed as user-facing capabilities
- Updater endpoints and the public key are empty, and updater artifact generation is disabled

### Security

- Typed native action boundary with path, URL, header, payload, delay, and nesting validation
- Bounded controller-event and action queues
- Cryptographic provenance record for the vendored SDL runtime and import libraries
