# Lanyard feature parity

This matrix maps the supplied Python application's operations to the Rust/React port. “Implemented” is a source coverage claim; it does not replace native desktop acceptance testing.

| Original feature | Port implementation |
| --- | --- |
| PIN setup, verification, change, manual lock | Rust master-key wrapping, unlock backoff, Settings PIN rotation, lock control/tray item |
| Base vault and project workspaces | Sidebar base/project navigation, per-project item counts and filters |
| Project create/edit/delete with item cascade | ProjectEditor, confirmation, transactional metadata commit and keychain cleanup |
| API keys, passwords, licenses, recovery codes, environments, cryptographic keys | Six typed categories, defaults and arbitrary named/multiline fields |
| Item create/edit/delete and project assignment | ItemEditor, ItemDetail, confirmation; move between projects and base vault |
| Reveal/hide and clipboard copy | Per-field visibility; native copy; conditional clipboard clearing after 30 seconds |
| Environment file import | File picker and Rust data-only .env parser, multiline quotes/comments/escapes; no command or variable expansion |
| Ed25519, RSA 2048 and RSA 4096 generation | Rust PEM generation in PKCS#8 / SubjectPublicKeyInfo; manual key paste remains available |
| Tray show/quit, close-to-tray | Native Tauri tray and persistent close preference |
| Random local port/discovery | Loopback-only dynamic port; protocol-v2 endpoint descriptor |
| Request a particular item or ask user to choose | Target/category validation, base/project optgroups, reason/name, visible timeout |
| Once, always, deny, expiry | Bounded pending queue, pairing-token grants, unlocked-vault requirement |
| Python client request_link/get_secret/get_field | SDK 0.2 retains public methods/return shapes and adds native identity storage |
| Theme presets | ULIX Dark default, ULIX Light and ten curated alternatives |
| New requested custom theme editor | Six editable palette colors, named persistent custom presets |
| New requested universal updater | Signed manifest/native artifact verification, release notes, download/install/retry |
| New permission controls | Forget paired apps or revoke individual automatic item grants |

The port deliberately removes the old unauthenticated name-only IPC protocol: legacy requests receive HTTP 426. No app can inherit another app's saved grants by choosing the same display name. Legacy imports retain item/project IDs but require re-pairing clients. All old vault/category operations have an implemented counterpart above.

The overall workspace remains a sidebar plus categorized vault/project content, with ULIX's black/white/emerald palette, Geist interface/technical type and Playfair display headings. Theme customization is the user-authorized exception to the default brand palette.
