# Coding Agent Guidance

## Project Context

Mind Mark is currently a documentation-stage project for a local-first bookmark manager. Read `requirements.md`, `high-level-plan.md`, `detailed-plan.md`, and `architecture.md` before making architectural or product-scope changes.

The agreed starting stack is Tauri 2, React/TypeScript, Rust, and SQLite/FTS5. This is a design baseline; do not claim components exist until they are implemented. The initial target is Windows, Chrome, and a local database.

## Product Constraints

- Initial Chrome intake is one-way and read-only. Never modify Chrome bookmark files or call Google account APIs unless the product scope is explicitly changed.
- Do not delete app bookmarks merely because they are missing from a Chrome snapshot.
- Make sync/import repeatable and idempotent using stable source identities.
- Keep Chrome folders distinct from Mind Mark categories and tags.
- Preserve original URLs and source provenance. Use conservative URL normalization for matching only.
- Duplicate merges require user confirmation and must preserve source links and user metadata.
- Full-text search is the initial search capability. Treat vector/semantic search, cloud storage, other browsers/OSes, and bidirectional sync as future work unless explicitly prioritized.
- Treat bookmarks as sensitive browsing history. Avoid logging full titles or URLs.

## Architecture Guidance

- Keep the application a modular monolith; do not add microservices or cloud requirements for local functionality.
- Keep domain rules independent of Tauri, React, Chrome paths, SQLite, and search implementations.
- Put use-case orchestration in the application layer, platform/file/database code in infrastructure adapters, and keep Tauri commands thin.
- Use versioned database migrations, foreign keys, short transactions, and a clear serialized write path for sync/import/merge operations.
- Treat search indexes and embeddings as derived data; canonical bookmark records remain authoritative.
- Add abstractions at real replacement boundaries, not speculatively throughout the codebase.
- Follow the actual generated project conventions once scaffolding exists; do not create placeholder files or invent build commands.

## Working Practices

- Inspect nearby code and tests before editing; keep changes focused and preserve existing user changes.
- For behavior changes, add or update focused tests for idempotent sync, profile isolation, tree preservation, import validation, merge safety, and index consistency as relevant.
- Validate with the narrowest available test, typecheck, lint, or build command. Use only commands defined by the project; if none exist yet, state that validation is unavailable rather than guessing.
- Update the relevant documentation when an architectural decision or supported behavior changes.
- Do not silently widen scope or change the agreed sync direction, privacy posture, or technology stack.
