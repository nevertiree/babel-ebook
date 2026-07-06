# Security Policy

## Supported Versions

Only the latest released version of BabelEbook is actively supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it privately by emailing the maintainer or opening a confidential issue. Do not disclose security issues publicly until they have been addressed.

Please include:

- A description of the vulnerability.
- Steps to reproduce it.
- The affected version(s).
- Any suggested mitigation or fix.

We aim to respond to security reports within 7 days.

## Security Best Practices for Users

- Never commit API keys to the repository. Use environment variables, the OS keyring, or local config files excluded by `.gitignore`.
- Keep your Rust toolchain and dependencies up to date.
- Only download installers from the official repository releases.
