# Tauri capabilities — security notes

This directory holds the capability JSON files Tauri loads at build time.
Edits here change the surface area exposed to the webview.

## Active permissions and known risks

### `opener:allow-open-path` (risk: Low)

Lets the webview call `openPath(<arbitrary path>)`, which delegates to the
OS default opener. Currently used by `lib/components/changes/StagingArea.svelte`
to open the AI-generated review file the user just clicked.

The risk is theoretical — a webview-side XSS would gain a way to open
arbitrary local files in their default app. Our CSP (`script-src 'self'`)
makes XSS unlikely, and the call is gated on a user click against a path
the backend already validated.

**Hardening plan (not blocking):** replace this permission with a custom
Tauri command (`open_repo_file(path)`) that validates `path` is inside
one of the open project roots and rejects traversal segments before
forwarding to the opener plugin. Tracking in the security audit doc.

### `dialog:allow-open` (risk: Low)

No path scope — the dialog plugin simply opens the system file picker,
which already requires a user click.

### `process:default` (risk: Low)

Exposes `restart` and `exit` only (no spawn). Acceptable.

### `updater:default` (risk: Medium)

Auto-updater fetches the manifest from a fixed URL and verifies signatures
with the embedded minisign pubkey. See `tauri.conf.json` for the endpoint;
key rotation is documented in the release runbook.

## When adding a new permission

1. Pick the most scoped variant available
   (`*:allow-foo-with-arg` over `*:default`).
2. Document the rationale in this file's "Active permissions" section.
3. If the permission is dual-use (read **and** write), prefer wrapping it
   behind a custom Rust command that validates inputs.
