# Git Flow Strategy

AetherDebugger follows a strict Git Flow branching model to ensure stability and high quality.

## Branches

- **`main`**: Production-ready code. Only merged from `release/` or `hotfix/`.
- **`develop`**: Integration branch for features. The source for `feature/` branches.
- **`feature/*`**: Individual features or bug fixes. Branch off `develop`, merge back via Pull Request.
- **`release/*`**: Preparation for a new production release. Branch off `develop`.
- **`hotfix/*`**: Urgent fixes for `main`. Branch off `main`.

## Pull Request Requirements

1. **Zero Warnings**: No clippy or rustfmt warnings allowed.
2. **Test Coverage**: All existing tests must pass. New features must include tests.
3. **Atomic Commits**: Small, meaningful commits.
4. **Review**: At least one approval from a maintainer.

## Committing

We follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation changes
- `chore:` maintenance/infra
- `refactor:` code change that neither fixes a bug nor adds a feature
