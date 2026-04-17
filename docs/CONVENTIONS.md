# Documentation Conventions

## Before writing

- Before creating a new document, check if an existing document already covers the topic and update it instead.
- Every new document must be added to the [docs/README.md](README.md) index with a one-line description.

## Style

- Write in present tense, active voice.
- Use Markdown for all documentation.
- Use relative links between documents (e.g., `[ARCHITECTURE.md](ARCHITECTURE.md)`).
- Keep headings descriptive and hierarchical (H1 for title, H2 for sections, H3 for subsections).

## Code examples

- Code examples in documentation must be tested or clearly marked as illustrative with a note such as:
  > **Illustrative** — this example shows the general pattern but is not extracted from a test suite.
- Use fenced code blocks with a language identifier (e.g., ` ```rust `, ` ```typescript `).

## File naming

- Use `UPPER_CASE.md` for top-level documents (e.g., `ARCHITECTURE.md`).
- Use `lower-case.md` for topic-specific guides (e.g., `stockfish-integration.md`).
