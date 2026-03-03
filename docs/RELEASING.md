# Releasing AetherDebugger

This document describes the process for creating a new release of AetherDebugger.

## Release Process

1.  **Update Version**:
    - Update the version in the root `Cargo.toml`.
    - Run `cargo check` to update `Cargo.lock`.
    - Commit and push to `main`.

2.  **Tag the Release**:
    - Create a signed tag (preferred):
      ```bash
      git tag -s v0.1.0 -m "Release v0.1.0"
      ```
    - Push the tag:
      ```bash
      git push origin v0.1.0
      ```

3.  **Automated CI**:
    - The `release.yml` workflow will trigger automatically on tags matching `v*`.
    - It will build binaries for Linux, Windows, macOS, and iOS.
    - It will create a GitHub Release draft or update an existing one with the compiled artifacts.

4.  **Finalize Release**:
    - Review the generated release on GitHub.
    - Add release notes summarizing the changes using the template at `docs/RELEASE_TEMPLATE.md`.
    - Publish the release.

## Checklist

- [ ] All CI checks passed on `main`.
- [ ] `CHANGELOG.md` is up to date.
- [ ] Documentation (`QUICKSTART.md`, `BUILD.md`) matches the new version.
- [ ] Hardware-in-the-Loop (HIL) tests passed for the release candidate.
