# Security Policy

## Supported Versions

BeardGit is currently maintained through the stable release line and the beta integration branch.

| Version / Channel | Supported |
| --- | --- |
| Latest stable release | :white_check_mark: |
| `beta` branch / pre-releases | Best effort |
| Older releases | :x: |

Security fixes are prioritized for the latest stable release. Fixes may be prepared on `beta` first when validation or integration work is required before publishing a stable release.

## Reporting a Vulnerability

Please do **not** report security vulnerabilities through public GitHub issues, pull requests, or discussions.

Use **GitHub Private Vulnerability Reporting** for this repository whenever possible:

1. Open the **Security** tab of the repository.
2. Choose **Report a vulnerability**.
3. Include the details listed below.

If GitHub Private Vulnerability Reporting is unavailable for you, contact the maintainer at:

**adolfo_fuentes@outlook.com**

## What to Include

To help validate and fix the issue, please include:

- A clear description of the vulnerability.
- Steps to reproduce the issue.
- The affected BeardGit version, commit, or release channel.
- Your operating system and version.
- Any relevant logs, screenshots, proof-of-concept code, or crash output.
- Whether the issue affects credentials, local repository data, forge API access, update delivery, bundled CLIs, AI provider execution, or filesystem access.
- Any known workaround or mitigation.

Please avoid sending real credentials, access tokens, private repository data, or other sensitive information unless strictly necessary. Redact secrets from logs before sharing them.

## Response Expectations

The maintainer will aim to:

- Acknowledge the report within 7 days.
- Confirm whether the issue is reproducible and in scope.
- Keep the reporter updated when there is meaningful progress.
- Coordinate a fix and release plan when the vulnerability is accepted.
- Credit the reporter if desired and if disclosure is appropriate.

Some reports may be declined if they are not reproducible, do not affect BeardGit, depend on unsafe local configuration, or describe behavior that is already documented and intentional.

## Disclosure

Please give the project a reasonable opportunity to investigate and release a fix before publicly disclosing the vulnerability.

When a fix is ready, disclosure may happen through a GitHub Security Advisory, release notes, or another appropriate channel depending on severity and user impact.
