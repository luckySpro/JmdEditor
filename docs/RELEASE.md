# Releasing JmdEditor

## One-time setup: the update key

In-app updates and required updates need one signing key. Keep the private key out of the repository.

1. Use the key you were given, or make a new one with `pnpm --filter jmdeditor-desktop tauri signer generate -w ~/.tauri/jmdeditor.key`. A new key also needs its public key (`~/.tauri/jmdeditor.key.pub`, one line) copied into `plugins.updater.pubkey` in `apps/desktop/src-tauri/tauri.conf.json`.
2. In GitHub → Settings → Secrets and variables → Actions, add `TAURI_SIGNING_PRIVATE_KEY` (the whole private key file) and, if the key has one, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
3. Back the key up. If it is lost, installed apps can no longer verify updates and everyone has to reinstall by hand.

Without the secret, releases still build, with a warning, but they have no updater packages and no policy.

## Making a release

1. Set the same version in `apps/desktop/src-tauri/tauri.conf.json`, `apps/desktop/package.json` and `Cargo.toml` (`workspace.package.version`), then merge.
2. Actions → Release → Run workflow, with `tag` (for example `v0.2.0`), optional `notes`, and `force` when every older version must update. Pushing a `v*` tag works too (no notes, no force).
3. The workflow builds macOS, Windows and Linux, uploads the installers, updater packages and `latest.json`, signs `policy.json`, and publishes the release only when all three succeed.
4. Set `DESKTOP_VERSION` in `apps/miniprogram/config.js` to the new version and upload the mini-program.

## Required updates

`policy.json` says which versions may still run:

```json
{ "version": "0.2.0", "minimum": "0.2.0", "notes": "…", "pub_date": "…" }
```

- `minimum` is taken from `apps/desktop/src-tauri/update-policy.json`, or set to the new version when `force` is ticked. Raise it in that file to keep a minimum across later releases.
- An app older than `minimum` shows an update dialog that cannot be closed. It saves the open note, installs the update and restarts. An app older than `version` only gets a notice.
- The policy is trusted only when its signature matches the key in the app, and the app remembers the highest minimum it has seen. A failed check never locks the app.
- Do not delete a published release or re-use a tag to undo a required update. Release a newer version instead.

Testing locally: `JMD_UPDATE_BASE=http://127.0.0.1:8099` points a debug build at a folder holding `policy.json`, `policy.json.sig`, `latest.json` and the update package.
