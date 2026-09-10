# Lanyard 0.2.0

ULIX's desktop credential vault, rebuilt with Tauri 2, Rust and React. Windows, macOS and Linux source/build paths are included. Python applications use SDK 0.2; any desktop application or CLI can use the documented local HTTP protocol.

## Development

Install Node 22+, stable Rust, and [Tauri's native prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS. Linux also needs an unlocked Secret Service keyring for real vault storage.

```
npm ci
npm run tauri dev
```

The default theme is ULIX Dark. Settings offers ULIX Light, ten additional presets, a custom palette editor, PIN rotation, app permission management, and software updates. Credentials stay in the native OS credential store, encrypted with a random master key wrapped by a PIN/passphrase-derived key. Metadata integrity is authenticated. No plaintext fallback is used if the credential store is unavailable.

## Existing Python desktop vault

On first launch, Lanyard detects the old `config.json` / `meta.json` in the original OS data location and offers import. Run under the same OS user/keychain account and supply the original PIN plus a new PIN/passphrase. Import verifies and decrypts every item before committing the new vault; the old vault files and keychain entries remain untouched. Project/item IDs are retained. Old grants based only on application names are intentionally discarded, so apps must pair again.

The old vault's UI preferences are not imported from its separate webview storage. You can select a preset or recreate a palette in the new theme editor. Do not delete the original vault until you have reviewed the imported credentials. A forgotten old PIN or unreadable old keychain entry cannot be bypassed by import.

## SDK and other languages

Upgrade `lanyard` to `>=0.2.0,<0.3` alongside the desktop app. Existing Python method names/return shapes remain available, but the protocol now requires a generated pairing identity stored in the client's OS credential store. Skudio 0.9.15 contains the matching integration.

See `docs/DESKTOP_API.md` for discovery, pairing, consent, errors and HTTP limits. The app listens only on loopback, rejects browser-originated requests and never exposes its inventory to external clients. Every item can be shared once or granted to a specific paired app while the vault is unlocked; revoke access in Settings.
