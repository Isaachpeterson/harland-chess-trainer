## Description

<!-- What does this PR do? Link any related issues. -->

## Type of change

- [ ] Bug fix
- [ ] New feature (slice implementation)
- [ ] Refactor (no behavior change)
- [ ] Documentation update
- [ ] CI/CD or build change

## Implementation slice

<!-- Which slice from docs/IMPLEMENTATION_PLAN.md does this PR belong to? -->

Slice: <!-- e.g. Slice 4 — Mistake Detection -->

## Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] Frontend: `npm run lint`, `npm run typecheck`, `npm run test` pass (if UI changed)
- [ ] No `unwrap()` / `expect()` added without a safety comment
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] `docs/ARCHITECTURE.md` updated (if crate or module structure changed)
- [ ] Version **not** bumped (version bumps are a separate release step)

## Screenshots

<!-- If this changes the UI, include before/after screenshots. Otherwise delete this section. -->

## Notes for reviewers

<!-- Anything reviewers should pay extra attention to? -->
