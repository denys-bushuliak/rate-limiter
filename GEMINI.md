
# Project Context

## Commit Message Guidelines

When generating commit messages, follow these principles:

1. **Title describes intent, not files** — e.g., "configure dev environment and refactor LeakyBucket" instead of "add devcontainer, agent config...".
2. **Each bullet explains the *why*** — always include purpose/benefit (e.g., "to support containerized Rust development", "for improved type safety").
3. **Use professional, action-oriented language** — prefer "Refactored", "Added to support...", "Optimized..." over flat "Added...".
4. **Keep it concise** — 5 bullets max, each one line.

Example:
```
chore: configure dev environment and refactor LeakyBucket

- Added `.devcontainer.json` to support containerized Rust development.
- Added `.agents/SKILL.md` to optimize agent tool performance by ignoring 'target/'.
- Created `GEMINI.md` for project-level instructions.
- Refactored `LeakyBucket` to include `PhantomData` for improved type safety.
- Initialized `package-lock.json` for workspace dependency management.
```
