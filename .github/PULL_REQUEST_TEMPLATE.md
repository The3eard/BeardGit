## Summary

<!-- Describe what this PR changes and why. -->

## Type of change

<!-- Mark all that apply. -->

- [ ] `feat:` user-visible feature
- [ ] `fix:` bug fix
- [ ] `docs:` documentation-only change
- [ ] `refactor:` internal restructuring without behavior change
- [ ] `test:` test-only change
- [ ] `chore:` maintenance, dependency, CI, or tooling change

## Target branch

- [ ] This PR targets `beta`.
- [ ] This PR does not target `main`.

## Related issues

<!-- Link related issues, discussions, or context. Use "Closes #123" when applicable. -->

## Testing

<!-- Mark what you ran. Explain anything that is not applicable. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `npx svelte-check`
- [ ] `npx vitest run`
- [ ] `npx stylelint "src/**/*.{svelte,css}"`
- [ ] `npx eslint src`
- [ ] Manual app testing on macOS / Linux / Windows

## UI, theme, and i18n checklist

- [ ] I used shared UI primitives where practical.
- [ ] I did not introduce hardcoded color literals outside the documented theme/brand-color sources.
- [ ] I added or updated strings in `messages/en-US.json` and `messages/es-ES.json` when user-facing text changed.
- [ ] I regenerated Paraglide bindings when editing message catalogs directly.

## Security and privacy checklist

- [ ] This PR does not log tokens, credentials, private repository content, or other sensitive data.
- [ ] This PR does not introduce telemetry, analytics, or unexpected network calls.
- [ ] Changes touching auth, credential storage, filesystem access, updater behavior, bundled CLIs, or AI provider execution are explained in the summary.

## CLA

- [ ] I have read and agree to `CLA.md`.

## Screenshots or recordings

<!-- Add screenshots or recordings for UI changes. -->

## Notes for reviewers

<!-- Call out risky areas, follow-up work, known limitations, or specific review requests. -->
