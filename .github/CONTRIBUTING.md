# Contributing to BabelEbook

Thank you for your interest in improving BabelEbook! This document provides guidelines for contributing to the project.

## Getting Started

1. Fork the repository and clone your fork.
2. Install [Rust](https://rustup.rs/) 1.78 or newer.
3. Install [pnpm](https://pnpm.io/) for the desktop frontend.
4. Build the workspace:

   ```bash
   cargo build --workspace
   cd desktop && pnpm install
   ```

## Development Workflow

- Create feature branches from `develop`.
- Keep commits focused and use conventional commit style when possible (`feat:`, `fix:`, `chore:`).
- Run the quality gates before submitting a pull request:

  ```bash
  cargo fmt -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cd desktop && pnpm exec tsc --noEmit && pnpm build
  ```

## Reporting Issues

When reporting bugs, please include:

- The version of BabelEbook (or commit hash).
- Steps to reproduce the issue.
- Expected vs. actual behavior.
- Your operating system and Rust version.

## Pull Request Process

1. Ensure all tests and lint checks pass.
2. Update `README.md` and `CHANGELOG.md` if your change affects user-facing behavior.
3. Keep the diff scoped to the feature or fix being introduced.

## Code of Conduct

Please be respectful and constructive in all project interactions.
