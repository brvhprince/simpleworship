# Releasing SimpleWorship

Installers for macOS, Windows and Linux are built by GitHub Actions
(`.github/workflows/release.yml`) when a version tag is pushed. They land on
a **draft** release; publishing the draft is what makes installed copies
offer the update.

## What gets built

| Platform | Files | Notes |
| -------- | ----- | ----- |
| macOS 11+ | `SimpleWorship_<version>_universal.dmg` | One app for Apple Silicon and Intel Macs. |
| Windows 10/11 | `SimpleWorship_<version>_x64-setup.exe`, `SimpleWorship_<version>_x64_en-US.msi` | The `.exe` installs for the current user (no admin needed); the `.msi` suits IT-managed computers. WebView2 is downloaded if missing. |
| Linux | `.AppImage`, `.deb`, `.rpm` | The `.deb` and `.rpm` pull in the GStreamer plugins for video. |

Each also gets a `.sig` file, and the release gets a `latest.json`: that
is what the app's updater reads.

## One-time setup

### 1. The update signing key (required)

The app only installs updates signed with the project's key. The public
half is in `apps/desktop/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`);
the private half was created on the maintainer's computer at
`~/.tauri/simpleworship-updater.key`.

- **Back up that file somewhere safe** (a password manager). If it is lost,
  copies already installed can never be updated automatically again.
- In the GitHub repository: *Settings › Secrets and variables › Actions*,
  add the secret `TAURI_SIGNING_PRIVATE_KEY` with the file's contents
  (`pbcopy < ~/.tauri/simpleworship-updater.key` copies it exactly). Add
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` only if the key has a password; the
  current key has none, and GitHub does not accept an empty secret.

To make a new key (and put its `.pub` contents in `tauri.conf.json`):

```sh
pnpm --filter @simpleworship/desktop tauri signer generate -w ~/.tauri/simpleworship-updater.key
```

### 2. Code signing (optional, recommended)

Without it, the installers work but the systems warn about them:

- **macOS** says the app is from an unidentified developer. The first time,
  people right-click the app, choose **Open**, then **Open** again.
- **Windows** SmartScreen says "Windows protected your PC": click **More
  info**, then **Run anyway**.

To sign:

- **macOS:** an [Apple Developer Program](https://developer.apple.com/programs/)
  membership ($99 a year). Create a *Developer ID Application* certificate,
  export it as a `.p12`, and add these secrets: `APPLE_CERTIFICATE` (the
  `.p12` as base64: `base64 -i cert.p12`), `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY` (e.g. `Developer ID Application: Your Name (TEAMID)`),
  `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password) and
  `APPLE_TEAM_ID`. The app is then signed and notarised.
- **Windows:** [Azure Trusted Signing](https://learn.microsoft.com/azure/trusted-signing/)
  (a few dollars a month). Create an account and certificate profile and an
  app registration that may sign, then add the secrets `AZURE_CLIENT_ID`,
  `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID` and the *variables*
  `AZURE_SIGNING_ENDPOINT` (e.g. `https://wus2.codesigning.azure.net`),
  `AZURE_SIGNING_ACCOUNT` and `AZURE_SIGNING_PROFILE`.

The workflow uses whatever is configured and builds unsigned otherwise.

## Making a release

1. Set the version everywhere:

   ```sh
   pnpm set-version 0.2.0
   ```

2. In `CHANGELOG.md`, give the new version a section saying what changed,
   in words a church operator understands. These notes are what people see
   when the update is offered.
3. Commit, tag and push:

   ```sh
   git commit -am "Release 0.2.0"
   git tag v0.2.0
   git push origin main v0.2.0
   ```

4. Wait for the **Release** workflow to finish (about 20 minutes), then
   open the draft release on GitHub. Download and try the installer for
   your computer.
5. Paste the CHANGELOG section into the release notes and **Publish**.
   Installed copies find it within a day (or straight away with
   Help › Check for Updates…).

A bad release can be withdrawn by deleting it (or marking it a draft
again): the updater only looks at the latest published release.
