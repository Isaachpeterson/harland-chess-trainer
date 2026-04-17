# Versioning Standard

> This document defines how version numbers are assigned in the Harland Chess Trainer project. It is authoritative: when deciding what the next version should be, follow the rules here rather than intuition. This document is read by both human contributors and AI coding assistants.

**Scheme:** Semantic Versioning 2.0.0, adapted for pre-1.0 projects
**Reference:** https://semver.org/spec/v2.0.0.html

---

## 1. Core Format

All versions follow the form:

```
MAJOR.MINOR.PATCH[-PRERELEASE]
```

Examples:
- `0.1.0` — first release
- `0.1.1` — patch release
- `0.2.0` — feature release
- `0.2.0-alpha.1` — pre-release build of upcoming 0.2.0
- `1.0.0-rc.1` — release candidate for 1.0.0
- `1.2.3` — post-1.0 stable release

**Tags in git use a `v` prefix:** `v0.1.0`, `v1.2.3`. This is what triggers the release workflow. The version in `Cargo.toml`, `package.json`, and `tauri.conf.json` does **not** include the `v`.

---

## 2. Pre-1.0 Rules (Current Phase)

The project is in the `0.x` range until it's considered production-stable (see Section 5). During this phase, standard SemVer is relaxed to reflect that the API and user-facing behavior are still evolving.

**While on `0.x`:**

| Bump | When to apply |
|------|---------------|
| `0.MINOR.0` | New features that the user can see, notice, or depend on. Slice completions from `IMPLEMENTATION_PLAN.md` that finish a phase of the roadmap. **Breaking changes are allowed here** — pre-1.0 users are expected to read release notes. |
| `0.MINOR.PATCH` | Bug fixes, performance improvements, documentation, and refactors that don't change behavior the user can observe. |

There is no `0.MAJOR` equivalent — breaking changes during pre-1.0 go in minor bumps with clear notes in `CHANGELOG.md`.

### Mapping to the roadmap

The roadmap in `copilot-instructions.md` Section 5 corresponds to minor versions:

- `0.1.0` — MVP (current target)
- `0.2.0` — Quality & Retention features
- `0.3.0` — Smarter Puzzles (counter-threat, themes, OAuth)
- `0.4.0` — Opening & Endgame Trainers
- `0.5.0+` — Platform Expansion

Patch versions fit between them as needed (`0.1.1`, `0.1.2`, etc.).

---

## 3. Post-1.0 Rules (Future Phase)

Once the project reaches `1.0.0`, strict Semantic Versioning applies:

| Bump | When to apply |
|------|---------------|
| `MAJOR.0.0` | **Breaking changes** — anything that forces users to change behavior, reimport data, or reconfigure. Any SQLite migration that cannot auto-upgrade. Any removal of a Tauri command that external scripts might depend on. |
| `N.MINOR.0` | **New features** that don't break existing functionality. New puzzle modes, new training features, new integrations. |
| `N.M.PATCH` | **Bug fixes and internal improvements** with no user-visible behavior change. |

**Breaking changes require a clear upgrade path in `CHANGELOG.md` and a prominent warning in the GitHub release notes.**

---

## 4. Decision Tree

When a change is ready to release, walk through these questions in order:

**1. Does this change require users to re-fetch data, re-analyze games, or manually migrate settings?**
- Pre-1.0: minor bump (`0.1.0` → `0.2.0`)
- Post-1.0: major bump (`1.0.0` → `2.0.0`)

**2. Does this change alter the SQLite schema in a way the auto-migration cannot handle silently?**
- Pre-1.0: minor bump
- Post-1.0: major bump
- *Even if automated, document the migration in the changelog.*

**3. Does this change add a user-visible feature (new UI page, new setting, new command, new puzzle type)?**
- Minor bump (either era).

**4. Does this change complete a slice from `IMPLEMENTATION_PLAN.md`?**
- If it's the last slice of the target version: minor bump.
- If it's one of several slices in the current version: no bump yet — changes accumulate under `[Unreleased]` in the changelog until the version is ready to cut.

**5. Is this change a bug fix, performance improvement, dependency update, or documentation change?**
- Patch bump.

**6. Is this change internal only — refactor, test-only, CI tweak, developer tooling?**
- No version bump. Commit to `main` and continue.

---

## 5. When to Cut `1.0.0`

The project moves to `1.0.0` when **all** of the following are true:

- The v0.1 feature set has been used by real users for at least one month without critical bug reports.
- The SQLite schema is considered stable — future changes will be additive or use explicit migrations.
- The Tauri command surface is considered stable — external tooling could reasonably depend on it.
- The author has confidence that future breaking changes are rare enough to justify strict SemVer.
- All items in the v0.4 roadmap are complete, *or* the author explicitly decides the current feature set is "1.0-worthy."

There is no rush. Most successful projects stay on `0.x` for a long time. Do not cut `1.0.0` to celebrate a milestone — cut it because users benefit from the stability guarantee.

---

## 6. Pre-release Identifiers

Used for builds that aren't production-ready but need to be tagged (for test releases, release candidates, beta testers).

Format: `MAJOR.MINOR.PATCH-LABEL.NUMBER`

| Label | Meaning |
|-------|---------|
| `alpha` | Early development, known incomplete. Not recommended for end users. |
| `beta` | Feature-complete for the target version, under testing. May have bugs. |
| `rc` | Release candidate. No new features; only critical bug fixes before the final release. |

Examples:
- `0.2.0-alpha.1` — first alpha of 0.2.0
- `0.2.0-beta.3` — third beta of 0.2.0
- `0.2.0-rc.1` — first release candidate of 0.2.0

Pre-releases sort correctly under SemVer: `0.2.0-alpha.1 < 0.2.0-beta.1 < 0.2.0-rc.1 < 0.2.0`.

**When to use them:**
- `alpha` and `beta`: only if the project ever has external testers. For a solo project with no outside testers, pre-releases are usually unnecessary — just work on a branch and tag the real release when ready.
- `rc`: worth using before every `MAJOR` release once post-1.0, and before any release that changes data formats.

---

## 7. Where the Version Lives

The version number appears in multiple places and **must** be kept in sync. A release is not valid if these disagree.

| Location | Format |
|----------|--------|
| `Cargo.toml` (workspace root, `[workspace.package]`) | `0.1.0` |
| Each crate's `Cargo.toml` (inherits from workspace) | `version.workspace = true` |
| `app/src-ui/package.json` | `0.1.0` |
| `app/tauri.conf.json` (`version` field) | `0.1.0` |
| `CHANGELOG.md` | `## [0.1.0] - YYYY-MM-DD` |
| Git tag | `v0.1.0` |
| GitHub Release title | `v0.1.0 — short descriptive name` |

The User-Agent string used by `lichess-client` is constructed from the Cargo version at compile time, so bumping `Cargo.toml` automatically updates it.

---

## 8. Release Checklist

Before tagging a release, in order:

1. **Confirm the bump is correct** by walking through the decision tree in Section 4.
2. **Update `CHANGELOG.md`:**
   - Move entries from `[Unreleased]` into a new version section with today's date in ISO format (`YYYY-MM-DD`).
   - Group entries under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security` per Keep a Changelog.
   - Create a fresh empty `[Unreleased]` section at the top.
3. **Update version in all locations listed in Section 7.** Use `cargo set-version` (from `cargo-edit`) or edit manually. Confirm with:
   ```
   grep -r "0.1.0" Cargo.toml app/tauri.conf.json app/src-ui/package.json
   ```
4. **Run the full test suite:** `cargo test --workspace`, `npm test`, `cargo clippy -- -D warnings`, `cargo fmt --check`.
5. **Run the app locally once** to confirm it still launches and the happy path works.
6. **Commit** the version bump with message `chore(release): v{VERSION}`.
7. **Tag** the commit: `git tag -a v{VERSION} -m "v{VERSION}"`.
8. **Push** the commit and tag: `git push origin main --tags`.
9. **Wait for GitHub Actions** `release.yml` to build artifacts.
10. **Review the draft GitHub Release** — confirm artifacts are attached, release notes match the changelog entry, and the version string is correct.
11. **Publish the release** (if the workflow creates a draft) or confirm it's public.

---

## 9. Changelog Discipline

`CHANGELOG.md` follows [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/).

Structure:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
as described in `docs/VERSIONING.md`.

## [Unreleased]

### Added
- (new entries go here)

### Changed

### Fixed

## [0.1.0] - 2026-MM-DD

### Added
- Initial MVP release. See `docs/IMPLEMENTATION_PLAN.md` for the full slice history.
- Lichess game fetching and local storage.
- Stockfish-based game analysis with Lichess-eval fallback.
- Blunder detection and puzzle generation.
- Chessground-based puzzle solving UI.
- Basic stats page.
```

**Every PR that merges to `main` should add at least one changelog entry under the appropriate `[Unreleased]` subsection.** If the change is internal-only (refactor, tests, CI), use an entry under an `### Internal` subsection or skip the changelog for that PR — but default to documenting.

**Entry style:**
- User-facing language, not implementation detail. "Added spaced repetition for missed puzzles" not "Added `schedule_review` function in puzzle-gen."
- Past tense, sentence case, ends with a period.
- Link to relevant issues or PRs when helpful.
- Breaking changes get a `**BREAKING:**` prefix.

---

## 10. Special Cases

### Database migrations

Every migration that adds a new table or column without touching existing columns: **patch-safe** (can ship in a patch release).

Every migration that modifies or drops an existing column: **minor bump minimum** (pre-1.0) or **major bump** (post-1.0). Always include the migration logic in the release; never ask users to manually alter their database.

Migrations are numbered sequentially (`0001_initial.sql`, `0002_evaluations.sql`, etc.) and the numbering is append-only — once a migration is released, its number is fixed forever.

### Dependency updates

- **Patch-level dependency updates** (e.g., `tokio 1.35.0` → `1.35.1`): no version bump needed, bundle into the next planned release.
- **Minor-level dependency updates** with no API impact: patch bump.
- **Major-level dependency updates** (e.g., `tauri 2.x` → `3.x`): minor bump (pre-1.0) or major bump (post-1.0). Treat this as a feature release with its own testing cycle.

### Stockfish bundled binary updates

The Stockfish version is tracked in the release notes. Updating to a newer Stockfish typically warrants a patch bump (no API change, just a stronger engine). If the new Stockfish changes UCI behavior in a way that affects the `engine` crate, treat as a minor bump.

### Security fixes

Security fixes get patch bumps, but release notes should prominently indicate the security nature of the fix. If the fix is critical (e.g., credential exposure), cut a patch release immediately rather than bundling with other changes.

### Yanked releases

If a release has a critical bug discovered immediately after publishing:

1. Publish a patch release with the fix as soon as possible.
2. Edit the broken release's GitHub Release to add a note at the top: `⚠️ This release has a known issue — please use vX.Y.Z+1.`
3. Do **not** delete the tag or the release — this breaks anyone who already downloaded.

---

## 11. For AI Coding Assistants

When making code changes that might warrant a version bump:

1. **Never silently edit version numbers.** If you think a version bump is needed, raise it as a separate step and get human confirmation.
2. **Always add a `[Unreleased]` changelog entry** as part of any user-visible change. This is part of the change, not a follow-up task.
3. **When uncertain about the bump level, consult Section 4's decision tree.** When still uncertain, propose the more conservative option (patch rather than minor, minor rather than major).
4. **Do not create version tags.** Tagging is a human action performed during the release checklist (Section 8).
5. **Do not modify `CHANGELOG.md` entries that are already under a published version heading.** Only `[Unreleased]` is editable.

A typical prompt response around versioning looks like:

> "I've implemented the feature and added an entry under `[Unreleased] → Added` in CHANGELOG.md. Per `docs/VERSIONING.md` Section 4, this is user-visible and completes Slice N, so when you're ready to release I'd recommend bumping to `0.2.0`. I have not changed any version numbers — that's your call."

---

## 12. Examples

### Scenario: Finished Slice 1 (Lichess fetch + storage)

- **Versioning action:** none. Slice 1 is one of ten slices in v0.1. Changes accumulate under `[Unreleased]`.
- **CHANGELOG entry:** "Added Lichess game fetching and SQLite storage."

### Scenario: Finished all v0.1 slices, ready to release

- **Versioning action:** bump to `0.1.0`.
- **Procedure:** Section 8 release checklist.

### Scenario: After v0.1 shipped, a user reports the sync button hangs on rate-limited responses

- **Versioning action:** fix the bug, bump to `0.1.1`.
- **CHANGELOG entry under `### Fixed`:** "Sync no longer hangs when Lichess returns HTTP 429; now retries with backoff."

### Scenario: v0.1 shipped, time to start v0.2 work on spaced repetition

- **Versioning action:** no bump yet. Changes accumulate under `[Unreleased]`. When v0.2 is feature-complete, bump to `0.2.0`.

### Scenario: A database migration changes the `puzzles` table to add a `difficulty` column

- **Pre-1.0 versioning action:** minor bump (`0.2.0` → `0.3.0`). Technically additive, but schema changes warrant the minor bump for safety.
- **Post-1.0 equivalent:** minor bump (`1.4.0` → `1.5.0`) — still additive, so not breaking.

### Scenario: A migration drops the `themes` column because it was never used

- **Pre-1.0 versioning action:** minor bump with a `**BREAKING:**` note in the changelog.
- **Post-1.0 equivalent:** major bump.

---

*Version numbers are a communication tool for users, not a scoreboard. Use them to tell users what to expect, not to signal project activity.*