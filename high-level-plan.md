# Mind Mark: High-Level Plan

## Product Goal

Mind Mark is a local-first desktop application for consolidating, organizing, and finding a large personal bookmark collection. The first release targets Windows, reads bookmarks from local Chrome profiles, and stores app-managed data in SQLite.

The initial product prioritizes safe, repeatable ingestion and reliable search over broad integrations. It must handle multiple thousands of bookmarks without requiring cloud services.

## Scope and Decisions

- **Initial platform:** Windows desktop.
- **Initial browser:** Chrome profiles available on the local machine.
- **Initial storage:** Local SQLite database.
- **Chrome sync:** One-way, read-only intake from Chrome into Mind Mark. Chrome's existing Google account sync remains Chrome's responsibility. Mind Mark does not write to Chrome's bookmark files or Google APIs.
- **Search:** Indexed full-text search first. Semantic/vector search is a later capability.
- **Profiles:** A Chrome profile is a bookmark source. Mind Mark profiles/workspaces are app-level scopes and should be modeled separately, even if initially mapped one-to-one.
- **Architecture recommendation:** A desktop modular monolith using Tauri 2, React/TypeScript, Rust, and SQLite. Confirm the stack before scaffolding; avoid distributed services for this local-first workload.

## MoSCoW Priorities

### Must Have

- Windows desktop application and local SQLite database with versioned schema migrations.
- Discover/select Chrome profiles and import their bookmark tree safely.
- Repeatable, idempotent, one-way sync; preserve source identity and report progress, warnings, and failures.
- Multiple Chrome source profiles with clear profile-scoped browsing and management.
- Browse and manage bookmarks while preserving Chrome folders/tree independently from app categories and tags.
- User-managed categories and tags.
- Indexed full-text search and filters, including profile, folder, category, and tag where applicable.
- JSON and CSV import/export with documented formats and validation/error reporting.
- Duplicate detection and a user-reviewed merge flow that preserves useful metadata and source provenance.
- Basic configuration, database/index health checks, and safe handling of concurrent reads and writes.

### Should Have

- Scheduled/background sync after user-triggered sync is proven reliable.
- Bulk operations, cross-profile search, and profile-scoped filters.
- Backup/restore and recovery guidance.
- Recovery from interrupted imports/syncs and explicit database/search-index consistency checks.

### Could Have

- Local semantic/vector search and an optional local embedding model.
- Assisted categorization and advanced collection insights.
- Browser extension or bidirectional sync with explicit conflict/deletion handling.
- Additional browsers, operating systems, and cloud metadata/vector storage.

### Won't Have in the Initial Release

- Editing Chrome's live bookmark file or calling Google account APIs.
- Automatically deleting duplicates or treating a bookmark missing from one Chrome snapshot as an instruction to delete app data.
- Cloud services as a runtime requirement, microservices, or distributed concurrency infrastructure.
- Unspecified arbitrary CSV compatibility; CSV import must use a documented format or explicit column mapping.

## Delivery Phases

1. **Foundation:** Confirm stack and UX assumptions; establish app shell, domain model, SQLite migrations, profile model, and test harness.
2. **Chrome intake:** Discover profiles, read a bookmark snapshot without modifying Chrome, preview/import the tree, and make repeated sync idempotent.
3. **Bookmark management:** Browse the tree, inspect/edit bookmarks, and manage categories and tags.
4. **Findability:** Add FTS5-backed text search, filters, and profile scoping.
5. **Data portability and cleanup:** Add JSON/CSV import/export, duplicate suggestions, and reviewed merging.
6. **Reliability:** Add health checks, progress/error reporting, backup/recovery, and concurrency/interruption hardening.
7. **Future extensions:** Evaluate scheduled sync, vector search, other browsers/platforms, and cloud storage against real usage needs.

## Success Criteria

- A user can import bookmarks from more than one Chrome profile without changing Chrome data.
- Repeating a sync does not create duplicate records; errors are visible and recoverable.
- Folder hierarchy, tags, categories, and source profile remain distinguishable and intact.
- Search and filtering work against the local index, and the index can be checked or rebuilt from canonical database data.
- JSON/CSV data can be exported and re-imported with documented, testable behavior.
- Duplicate merging is reviewable and does not silently lose bookmarks or provenance.

## Key Risks to Resolve During Design

- Chrome profile discovery and snapshot consistency when Chrome is running.
- What happens when a source bookmark changes or disappears; initial policy should not silently delete app records.
- Exact URL normalization and duplicate criteria, especially for tracking parameters and similar titles.
- CSV schema/column mapping and behavior for malformed or partial imports.
- Whether app-level workspaces are needed in addition to multiple Chrome source profiles.
- UI framework and accessibility expectations before the project is scaffolded.
