# Contributing to Dual Deck

Thank you for helping improve Dual Deck. Contributions should keep the application dependable,
lightweight, accessible, and understandable.

Participation in this project is governed by the [Code of Conduct](CODE_OF_CONDUCT.md). By
submitting a contribution, you agree that it may be distributed under the repository's
[MIT License](LICENSE). Third-party material must retain its own license and attribution.

## Before starting

- Search the [existing issues](https://github.com/sushi-dev55/Dual-Deck/issues) and
  [pull requests](https://github.com/sushi-dev55/Dual-Deck/pulls).
- Open an issue before starting a large feature, new controller backend, or architectural change.
- Keep each pull request focused on one concern.
- Do not include controller firmware, proprietary Sony artwork, credentials, signing material, or
  private user data.
- Do not add analytics, telemetry, or a network dependency without prior project discussion.

## Local setup

Development currently requires Windows, the `x86_64-pc-windows-gnu` Rust target, MSYS2 MinGW-w64,
Node.js 24, and pnpm 10. Follow the full setup in [README.md](README.md), then run:

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

Use a first-party DualSense for controller changes. The browser-only preview cannot verify native
input, persistence, tray behavior, or action execution.

## Quality requirements

Before opening a pull request, run:

```powershell
pnpm format:check
pnpm check
pnpm test
pnpm build
pnpm cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
pnpm cargo check --manifest-path src-tauri/Cargo.toml --locked --all-targets --target x86_64-pc-windows-gnu
$env:RUSTFLAGS = "-Dwarnings"
pnpm cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --target x86_64-pc-windows-gnu
Remove-Item Env:RUSTFLAGS
pnpm cargo test --manifest-path src-tauri/Cargo.toml --locked --all-targets --target x86_64-pc-windows-gnu
```

The `pnpm cargo` and `pnpm tauri` scripts place Rust build artifacts in the per-user
`%LOCALAPPDATA%\DualDeck\cargo-target` directory. Keep `DUALDECK_CARGO_TARGET_DIR` absolute and
free of whitespace if you override that location. The scripts use `C:\msys64\mingw64\bin` for
`gcc.exe` and `dlltool.exe` by default. Set `DUALDECK_MINGW_BIN` to an absolute, whitespace-free
`mingw64\bin` directory when MSYS2 is installed elsewhere.

Add or update automated tests whenever behavior can be verified without hardware. A change is not
complete merely because it compiles.

## Hardware changes

Changes that affect controller input should be tested with a first-party DualSense over every
affected connection type. Record results in [docs/hardware-test-matrix.md](docs/hardware-test-matrix.md)
with the Windows build, controller firmware, SDL version, and exact behaviors exercised.

Relevant controller and runtime changes should cover:

- Discovery, disconnect, reconnect, and additional-device handling
- Every affected button, trigger, or axis
- A hidden editor window and another application in the foreground
- Mapping pause and resume
- Sleep and resume
- Battery and connection metadata when available
- Pass-through behavior in a controller-aware foreground application

Do not report an untested row as passing. Hardware unavailable to the contributor should remain
marked **Not run** and be called out in the pull request.

## Interface changes

Interface changes must remain usable with keyboard navigation, visible focus, Windows display
scaling, high contrast, and reduced motion. Include screenshots at the default window size and at
the minimum supported size. Avoid animation that obscures state or blocks input.

## Pull requests

Include:

- A concise description of the behavior change and its reason
- Automated and manual verification performed
- Screenshots or a short recording for visible interface changes
- Hardware matrix updates for controller behavior changes
- Compatibility, migration, dependency, privacy, and security considerations

Opening a pull request does not guarantee that a change will be merged. Maintainers may request a
smaller scope, additional evidence, design changes, or follow-up work to protect compatibility and
the release boundary.

Do not commit generated installers, local databases, logs, secrets, signing keys, editor-specific
files, or unreviewed third-party binaries.

## Dependencies and vendored files

Prefer maintained dependencies with clear licenses. Explain why a new runtime dependency is needed
and check its release artifacts and license before committing it.

Every vendored binary must include its exact upstream version, source URL, license, upstream archive
checksum, and committed-file checksums. Update that record in the same pull request as the binary.

## Reporting bugs

Use the [bug report form](https://github.com/sushi-dev55/Dual-Deck/issues/new?template=bug_report.yml).
Include the Dual Deck version, Windows version, connection type, controller firmware version when
known, reproduction steps, expected behavior, and actual behavior. Remove personal paths, webhook
credentials, passwords, and other secrets from logs before attaching them. Report vulnerabilities
through [SECURITY.md](SECURITY.md), not through a public issue.
