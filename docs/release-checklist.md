# Release checklist

This is the release gate for the current public Windows installer. Automatic updates are not a
current user-facing capability and have a separate future gate below.

The public repository is `sushi-dev55/Dual-Deck`. It intentionally has no installer-publishing
workflow yet because a Windows signing provider and protected release process have not been
configured. An updater key is required only when automatic updates enter release scope.

## Repository identity

- [x] Confirm `sushi-dev55/Dual-Deck` as the public GitHub repository.
- [ ] Confirm `Dual Deck` as the public product name and check for naming conflicts.
- [x] Confirm repository URLs, package author information, and installer copyright metadata all use
      the approved public identity.
- [x] Confirm the MIT copyright holder and year in `LICENSE` and package metadata.
- [x] Confirm the reverse-domain Tauri identifier is owned and final.
- [ ] Enable GitHub private vulnerability reporting.
- [ ] Protect the default branch and require the CI workflow.

## Version and source

- [ ] Set the first stable release version to `1.0.0` after every required gate passes.
- [ ] Set the same version in `package.json`, `src-tauri/Cargo.toml`, and
      `src-tauri/tauri.conf.json`.
- [ ] Move completed entries from **Unreleased** into a dated `CHANGELOG.md` release section.
- [ ] Build from a clean clone of the exact release commit with frozen and locked dependencies.
- [ ] Confirm frontend and Rust dependency reviews are complete.
- [ ] Verify the SDL version, license, upstream archive hashes, and vendored file hashes against
      `src-tauri/vendor/sdl3/DEPENDENCY.md`.
- [ ] Confirm no database, logs, credentials, signing files, or private paths are tracked.

## Automated quality gates

- [ ] `pnpm format:check` passes.
- [ ] `pnpm check` passes with no Svelte or TypeScript errors.
- [ ] Frontend tests pass.
- [ ] `pnpm build` passes.
- [ ] Rust formatting passes.
- [ ] Rust check passes for `x86_64-pc-windows-gnu`.
- [ ] Rust tests execute and pass with the vendored SDL DLL on `PATH`.
- [ ] The CI workflow passes on the release commit.

## Manual application quality

- [ ] Fresh install succeeds on Windows 10 64-bit.
- [ ] Fresh install succeeds on Windows 11.
- [ ] Upgrade over the previous stable version preserves profiles and settings.
- [ ] Uninstall removes application files without deleting unrelated user files.
- [ ] First launch, startup launch, start minimized, close to tray, restore, and explicit quit work.
- [ ] A second launch focuses the existing instance.
- [ ] Profile create, rename, duplicate, activate, mapped switching, and delete work.
- [ ] Press, Release, Long press, Double press, and Hold work for every supported single-button
      control.
- [ ] Every shipped action succeeds and reports invalid configuration clearly.
- [ ] Offline startup and normal offline use do not stall the interface.
- [ ] Minimum window size, keyboard navigation, focus, display scaling, high contrast, and reduced
      motion are verified.

## Hardware gate

- [ ] Complete all release-required USB rows in `docs/hardware-test-matrix.md` on Windows 10.
- [ ] Complete all release-required USB rows in `docs/hardware-test-matrix.md` on Windows 11.
- [ ] Complete the Bluetooth rows or explicitly remove Bluetooth from release claims.
- [ ] Verify additional controllers are ignored without disrupting the active controller.
- [ ] Verify disconnect, reconnect, sleep, resume, pause, and hidden-window behavior.
- [ ] Verify physical input still reaches a foreground controller-aware application.
- [ ] Record controller firmware, Windows builds, and SDL version used for sign-off.

## Privacy and security

- [ ] Confirm there are no accounts, analytics, telemetry, or undeclared network requests.
- [ ] Review Tauri capabilities and remove unused plugin permissions.
- [ ] Review the content security policy against the production build.
- [ ] Test malicious paths, URLs, webhook headers, payload sizes, and malformed desktop commands.
- [ ] Confirm logs and error messages do not expose typed text, webhook secrets, or private paths.
- [ ] Review dependency advisories and resolve release-blocking findings.

## Future automatic-update gate

This section is not part of the current installer release gate. The updater plugin is registered,
but its endpoints and public key are empty, there is no user-facing update flow, and
`createUpdaterArtifacts` is `false`. Do not advertise automatic updates or publish updater metadata
until the feature is implemented and every item in this section passes.

Tauri updater signing verifies that an update came from the release process. It is separate from
Windows Authenticode signing.

- [ ] Generate the final Tauri updater key pair in a controlled environment.
- [ ] Store the private key and password as protected GitHub environment secrets.
- [ ] Back up the private key in a separate protected location. Losing it prevents trusted updates
      for existing installations.
- [ ] Put only the updater public key in `src-tauri/tauri.conf.json`.
- [ ] Configure the HTTPS updater endpoint for releases published by `sushi-dev55/Dual-Deck`.
- [ ] Require approval for the protected release environment.
- [ ] Generate and sign updater artifacts from the final release commit.
- [ ] Verify the generated update metadata, platform, architecture, URL, signature, and checksum.
- [ ] Test a signed update from the previous stable release on a clean Windows 10 machine.
- [ ] Test the same update on Windows 11.
- [ ] Verify a modified artifact and invalid signature are rejected.

## Windows code signing

- [ ] Select a Windows code-signing provider or complete the open-source SignPath Foundation
      onboarding process.
- [ ] Record the approved publisher identity without storing private key material in the repository.
- [ ] Sign the application executable and NSIS installer.
- [ ] Verify signatures and timestamp chains on a machine that does not have development
      certificates installed.
- [ ] Confirm the displayed publisher and product identity are correct.
- [ ] Scan the signed artifacts and review Windows reputation warnings before publishing.

## Packaging and publication

- [ ] Build the current-user NSIS installer on a clean Windows runner.
- [ ] Confirm WebView2 installation behavior with and without an existing runtime.
- [ ] Confirm `SDL3.dll`, `licenses/SDL3.txt`, and `licenses/THIRD_PARTY_NOTICES.md` are present in
      the installed application.
- [ ] Test install and uninstall paths containing spaces and non-ASCII characters.
- [ ] Generate SHA-256 checksums for every public artifact.
- [ ] Create a signed Git tag that matches the application version.
- [ ] Draft release notes from the changelog with supported Windows versions and the unverified
      Bluetooth, automatic-update, import/export, backup, multi-action, and native OBS limitations.
- [ ] Attach the signed installer, checksums, and license notices to one GitHub Release.
- [ ] Verify every download from the public release page on a separate machine.
- [ ] Publish only after installer tests pass against that exact download.

## After publication

- [ ] Confirm a clean installation from the public release.
- [ ] Confirm the application makes no updater request and does not present an update status.
- [ ] Monitor private security reports and public issue reports for release regressions.
- [ ] Keep the previous signed installer, checksums, and release notes available for investigation
      and rollback.
- [ ] If a release must be withdrawn, remove its public installer before announcing remediation.
