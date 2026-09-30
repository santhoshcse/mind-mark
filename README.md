# Mind Mark

Mind Mark is a local-first Windows desktop app for importing, organizing, and managing a large bookmark collection. The initial source is local Chrome profiles and the app database is SQLite.

## Current Status

The Tauri 2 + React/TypeScript + Rust scaffold is in place. The initial SQLite schema and startup initialization, read-only Chrome profile discovery/sync, profile/folder navigation, and bookmark create/edit/archive with categories and tags have been implemented. Search, JSON/CSV import/export, deduplication, and health tools remain future phases.

The React/TypeScript production build passes. Core Rust tests passed for migrations and Chrome parsing/idempotent sync. A bookmark CRUD test exposed a mutex deadlock that has been repaired, but that specific test has not yet been rerun. A full Tauri build is not currently verified: the available WSL environment is missing GTK/WebKit development dependencies, and the Windows Rust toolchain is unavailable.

## Initial Scope

- **Platform:** Windows desktop.
- **Browser:** Chrome profiles on the local machine.
- **Storage:** Local SQLite with versioned migrations and FTS5 indexing.
- **Sync:** One-way, read-only intake from Chrome snapshots. Chrome remains responsible for Google account sync; Mind Mark does not write to Chrome files or call Google account APIs.
- **Profiles:** Chrome sources map to separate Mind Mark app profiles; app profiles can also be created independently.
- **Search:** Full-text search is planned; semantic/vector search is deferred.

## Implemented in the Foundation

- Discover local Chrome `Default` and `Profile N` directories containing bookmark data.
- Parse nested Chrome bookmark folders and preserve stable source IDs and sibling order.
- Repeat sync without duplicating bookmarks; report malformed and missing/stale source entries without deleting app records.
- Store app profiles, sources, folders, bookmarks, tags, categories, and sync runs in SQLite.
- Browse profiles and folders; create, edit, and archive bookmarks while managing tags and categories.

These features are early implementation and have not yet been validated in a complete Windows desktop runtime.

## Development

Install frontend dependencies and build the React application from the repository root:

```sh
npm install
npm run build
```

Run the Rust core tests in WSL from this repository's mounted path:

```sh
cargo test --manifest-path src-tauri/Cargo.toml -p mind-mark-core
```

The Tauri desktop runtime additionally requires platform prerequisites. For Linux/WSL, install the Tauri-documented GTK, WebKitGTK, and `pkg-config` development packages before attempting a full desktop build. Windows packaging should be built with the Windows Tauri/Rust prerequisites.

## Data Safety

- Chrome access is read-only in the initial release.
- Missing items in a Chrome snapshot are reported, not deleted.
- Sync is keyed by stable Chrome source IDs and should be repeatable.
- Original URLs and source provenance are retained; merges are not yet implemented.
- Bookmark content is private browsing history; avoid logging titles and full URLs.

## Documentation

- [Requirements](requirements.md)
- [High-level plan](high-level-plan.md)
- [Detailed plan](detailed-plan.md)
- [Architecture](architecture.md)
- [Agent guidance](AGENTS.md)
- [Claude Code guidance](CLAUDE.md)

Future scope includes other browsers and operating systems, cloud metadata/vector storage, semantic search, scheduled sync, and bidirectional sync with explicit conflict and deletion rules.
