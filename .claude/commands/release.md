---
name: release
description: Release a new version of mxpm
argument-hint: "<new-version>"
---

# Release mxpm

Release a new version of the Maxima Package Manager CLI.

## Arguments

`$ARGUMENTS` is the new version number (e.g. `0.2.0`). If not provided, ask the user.

## Pre-flight checks

1. Verify on `main` branch.
2. Verify the working tree is clean (no uncommitted changes).
3. Read `Cargo.toml` to get the current version.
4. Confirm the new version is different from the current version.
5. Check that the git tag `v<new-version>` does not already exist.
6. Run `cargo fmt --check`, `cargo clippy`, and `cargo test` — abort if any fails.

If any check fails, report it and stop.

## Steps

### 1. Update version

- Update `version` in `Cargo.toml` to the new version.

### 2. Update lockfile

- Run `cargo check` to update `Cargo.lock` with the new version.

### 3. Update CHANGELOG.md

- Read `CHANGELOG.md`.
- Rename the `## [Unreleased]` heading to `## [<new-version>]` with today's date.
- Add a fresh empty `## [Unreleased]` section above it with subsections `### Added`, `### Changed`, `### Fixed`.
- Show the user the changelog diff and ask for confirmation before continuing.

### 4. Commit

- Stage: `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`
- Commit message: `Release v<new-version>`
- Do NOT use `--no-verify`.

### 5. Tag

- Create an annotated tag: `git tag v<new-version>`

### 6. Push

- Ask the user for confirmation before pushing.
- Push: `git push origin main && git push origin v<new-version>`

### 7. Summary

Print a summary: version, tag, and remind the user that the GitHub Actions release workflow will build and publish binaries.
