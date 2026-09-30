# Mind Mark

Mind Mark is a planned local-first desktop application for importing, organizing, and searching a large bookmark collection. The initial target is Windows, local Chrome profiles, and SQLite.

> **Project status:** Planning stage. The application has not been scaffolded yet. The stack and architecture below are the agreed starting direction, not implemented features.

## Goals

- Safely bring bookmarks from one or more local Chrome profiles into Mind Mark.
- Preserve the imported folder tree while adding independent categories and tags.
- Make a collection of several thousand bookmarks practical to search and manage.
- Keep bookmark data local by default and make imports, merges, and recovery understandable.
- Leave clear extension points for future browsers, operating systems, storage providers, and bidirectional sync.

## Initial Scope

- **Platform:** Windows desktop.
- **Browser source:** Chrome profiles on the local machine.
- **Storage:** Local SQLite database.
- **Sync:** One-way, read-only intake from Chrome. Chrome continues to handle its own Google account synchronization. Mind Mark will not write to Chrome's bookmark files or use Google account APIs in the initial release.
- **Search:** Indexed full-text search first; vector/semantic search is deferred.

Planned first-release capabilities include bookmark-tree browsing, multi-profile source management, categories, tags, filters, JSON/CSV import and export, duplicate suggestions with user-reviewed merge, configuration, health checks, and safe concurrent access.

## Architecture Direction

The proposed stack is Tauri 2, React/TypeScript, Rust, and SQLite with FTS5. The application should begin as a modular monolith: keep domain and application rules separate from the desktop UI, Chrome file access, persistence, and search adapters. See [architecture.md](architecture.md) for boundaries, data semantics, and the proposed project structure.

## Documentation

- [Requirements](requirements.md): initial product requirements and future scope.
- [High-level plan](high-level-plan.md): MoSCoW priorities and delivery phases.
- [Detailed plan](detailed-plan.md): implementation sequence and acceptance checks.
- [Architecture](architecture.md): technical design and extension points.
- [Claude Code guidance](CLAUDE.md): repository-specific expectations for coding work.

## Development

The repository is currently documentation-only. There is no application source, package manifest, test suite, or build command yet. Development setup instructions should be added when the agreed stack is scaffolded and its actual commands are known.

## Data Safety Principles

- Never modify Chrome's live bookmark files in the initial release.
- A bookmark absent from a Chrome snapshot must not silently delete app data.
- Repeated imports should be idempotent and report their results.
- Keep original URLs and source provenance; use normalized values only for matching.
- Require user review for merges and avoid silent destructive deduplication.
- Treat bookmark data as private browsing history; do not include full titles or URLs in routine logs.

## Future Scope

Potential later work includes other browsers and operating systems, cloud metadata/vector storage, bidirectional sync with explicit conflict/deletion handling, scheduled sync, and semantic/vector search. These are not prerequisites for the initial local release.
