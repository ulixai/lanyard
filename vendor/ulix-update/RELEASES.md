# Release operations

The delivered app sources use the public Ed25519 key already embedded in Skudio for manifest verification. The website uses `ULIX_UPDATES_PRIVATE_KEY`, falling back to `SKUDIO_PRIVATE_KEY`. Keep the fallback only when it is the private half of that public key. The existing legacy Skudio route always uses its original `SKUDIO_PRIVATE_KEY`, independently of the new service.

Artifact signing is separate. No production private key is included, and artifact public keys are intentionally unconfigured. Production preflight fails until you configure them. Never publish a desktop build with a temporary key: that key becomes its future update trust anchor.

## One-time signing setup

Generate and securely back up a Tauri updater key on your development machine using `npm run tauri signer generate -- -w /private/path/ulix-updater.key`. Keep its private file/password in CI secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. The generated `.pub` file contains a base64-encoded Minisign public key; put its full content in the repository variable `ULIX_ARTIFACT_PUBLIC_KEY` or run:

```
python vendor/ulix-update/tools/configure.py . --artifact-key-file /private/path/ulix-updater.key.pub
```

If you choose a new manifest key instead of the Skudio key, set its private Ed25519 PEM in the website's `ULIX_UPDATES_PRIVATE_KEY` and provide the base64 raw 32-byte public key using `ULIX_MANIFEST_PUBLIC_KEY` or `configure.py --manifest-key`. Do not rotate either trust anchor without first shipping a migration build trusted by existing clients.

Tauri updater signatures establish update authenticity. Windows Authenticode and Apple Developer ID/notarization are separate publisher/platform controls. Keep the working DigiCert setup from your existing pipeline and initialize it before the Tauri build. Configure `bundle.windows.signCommand` with its signing command and Tauri's `%1` file placeholder. Sign the portable helper and Ulysses runtime too, before collecting artifacts. Do not modify any executable after its updater signature/digest has been generated.

For macOS, provide the Developer ID certificate export and password plus signing identity through `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`; notarization also needs `APPLE_ID`, `APPLE_PASSWORD` (app-specific), and `APPLE_TEAM_ID`, or Tauri's supported App Store Connect credentials. Sign nested runtime executables and the portable helper before packaging; notarize the distributed portable ZIP and staple the `.app` before final hashing. The workflow template passes the standard Tauri signing variables, but your Apple identity and DigiCert bootstrap must be configured in the repository. Run a signing-only credential check before spending time on model/runtime builds.

## Website deployment

1. Back up the database and apply `drizzle/0002_universal_software_releases.sql` using the project's normal migration command (`npm run db:migrate`). It adds the `SoftwareRelease` table/index; it does not repurpose Skudio's existing release table.
2. Set `ULIX_RELEASES_WEBHOOK_SECRET` to a long random publisher credential and configure the manifest private key as above. Keep publisher credentials out of clients and frontend environment variables.
3. Deploy the website normally. No database migration or production deployment was run while preparing these files.
4. Sign in to `/admin/releases` to publish, withdraw, or reactivate releases. Each record is scoped to product, channel, platform, architecture, edition, and version. Reusing an identity with different bytes is rejected. Publish a new version instead.

`/api/app-status` and the legacy Skudio webhook remain compatible. Legacy notifications are limited to global notices and Skudio notices. New clients use the universal route. New product slugs do not require another route or database schema change.

## Build and publish

Each app includes `.github/workflows/desktop-release.yml`, copied from the reusable template. It builds artifacts for Windows x64, macOS ARM64/Intel, and Linux x64/ARM64 when manually dispatched. Supply the final HTTPS download directory; artifact names include product/version/platform/architecture. Ulysses additionally creates portable ZIPs.

The workflow is a build template, not evidence that all native packages have been tested. Configure platform signing before running it for public releases. It uploads reviewable workflow artifacts only. It neither creates a public GitHub release nor publishes a website release automatically.

For a local build, configure keys, build Ulysses's runtime when applicable, run `preflight.py . --release`, build the helper for the same target, then `npm run tauri build -- --target TARGET --bundles FORMAT`. `collect-release.py` collects signed native artifacts and produces `installed.json` / `portable.json` records with hashes and sizes. Linux initial installs use AppImage plus the per-user install script. Windows portable users need the Microsoft WebView2 runtime; the installed NSIS edition can install its prerequisite.

Ulysses runtime source is pinned in `tools/build-runtime.py`. CPU is the compatibility default on Windows/Linux; Metal is the Mac default. `--backend vulkan` or `--backend cuda` retains a GPU build path when the corresponding SDK/toolkit is installed. Models are never shipped in update artifacts. Put them in the displayed Models folder, or `models/` beside a portable installation. Test model formats against the pinned runtime before changing the pin.

Upload artifacts without changing their bytes to the URLs in the records, then review and publish:

```
python vendor/ulix-update/tools/publish.py release-output/windows-x86_64/installed.json --dry-run
python vendor/ulix-update/tools/publish.py release-output/windows-x86_64/installed.json
```

The second command requires `ULIX_RELEASES_WEBHOOK_SECRET` in its environment and performs the publication. Up to 32 records may be published atomically. The same JSON can be pasted into the admin page instead.

## Required release validation

On each native platform: install the current edition; publish a signed staging-channel update with a higher version; check/download/install/relaunch; verify history, credentials and settings; withdraw the release and confirm installation is rejected. Test denied permissions, offline failures, a bad manifest key, a corrupted artifact, and a missing signature. On Ulysses portable, verify models/data remain byte-identical and interrupt the helper between file moves to exercise recovery. On Lanyard, import a copy of a real legacy vault under the same OS keychain account, then exercise all six categories, app consent/revocation, PIN rotation and tray behavior. Validate Mac keychain prompts, Windows Credential Manager migration, and Linux Secret Service on actual desktops.

Primary references: [Tauri updater](https://v2.tauri.app/plugin/updater/), [Windows signing](https://v2.tauri.app/distribute/sign/windows/), [macOS signing](https://v2.tauri.app/distribute/sign/macos/), [llama.cpp build instructions](https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md), [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
