# Contributing to AetherDebugger

We follow a **Trunk-Based Development** strategy.

## Branching Model

- **`main`**: The single source of truth. Always deployable.
- **Feature Branches**: Short-lived branches for new features or fixes.
  - Name format: `username/feature-name` or `topic/feature-name`.
  - Do NOT use long-lived `develop` or `release` branches.

## Workflow

1. **Branch off `main`**:
   ```bash
   git checkout main
   git pull origin main
   git checkout -b my-new-feature
   ```

2. **Commit often**:
   - Use [Conventional Commits](https://www.conventionalcommits.org/).
   - Keep commits small and atomic.

3. **Open a Pull Request**:
   - Target `main`.
   - Ensure specific checks pass (CI, formatting, tests).
   - Get at least one approval.

4. **Merge**:
   - Squash and merge is preferred to keep history clean.
   - Delete your branch after merging.

## Release Process

Releases are created by tagging `main`.
```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```
