# Security policy

## Supported versions

Dual Deck `0.1.0` is pre-release software and has not published a stable installer. Security fixes
are currently applied to the default branch. After `1.0.0`, the latest stable release will receive
security fixes.

| Version                 | Supported    |
| ----------------------- | ------------ |
| Default branch          | Yes          |
| Unreleased local builds | No guarantee |

## Reporting a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/sushi-dev55/Dual-Deck/security/advisories/new).
Include affected versions, reproduction steps, impact, and a suggested mitigation when available.

If private vulnerability reporting is unavailable, open a minimal public issue asking the
maintainers to enable a private reporting channel. Do not include vulnerability details in that
issue.

Use the [public issue tracker](https://github.com/sushi-dev55/Dual-Deck/issues) only for ordinary
bugs that do not create a security or privacy risk.

Do not open a public report for a vulnerability that could expose credentials, execute unintended
actions, compromise the host computer, or bypass a future update-verification boundary. Remove
passwords, tokens, signing material, private paths, and personal data from supporting files.

The maintainers will coordinate validation, remediation, and disclosure after receiving a report.
Receipt does not guarantee that a report is a vulnerability or that a particular release date can
be offered. Do not test against systems, devices, or accounts you do not own or have permission to
use.

## Security boundaries

Dual Deck treats dropped paths, URLs, webhook fields, controller events, and data crossing the
webview boundary as untrusted input. Native actions are represented as typed values and validated
by the Rust backend before execution.

Profile import/export and backup have no production implementation. Any future file-import path
must bound the file before deserialization, validate mapping counts and action-tree complexity,
assign new identifiers, and require an explicit review before mappings can be enabled.

Dual Deck does not expose unrestricted shell execution through profiles. It does not hide physical
controller input, install a kernel driver, or create a virtual controller. Controller input remains
available to the foreground application.

Profiles may contain sensitive text, local paths, webhook headers, and endpoint URLs. The local
database, a manually copied database file, and any future export or backup should therefore be
treated as sensitive.

The registered updater plugin has no endpoint or public key, updater artifacts are not generated,
and no user-facing update flow exists. Automatic updates must remain disabled until the updater
public key, HTTPS release endpoint, signed metadata, Windows code-signing identity, protected
release process, and update review are all in place.
