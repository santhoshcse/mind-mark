# Mind Mark: Detailed Plan

## 1. Architecture

### Recommended Stack

- **Desktop shell:** Tauri 2.
- **UI:** React and TypeScript.
- **Application core:** Rust.
- **Persistence:** SQLite with versioned migrations and FTS5 for initial text search.

This stack is a recommendation for a Windows-first desktop application with a plausible path to other desktop operating systems. Keep the application as a modular monolith. Do not add network services, cloud dependencies, or separate deployable services to solve a local-database problem.

### Architectural Boundaries

Keep business rules independent from Tauri commands, React components, Chrome file paths, and SQLite details.

- **Domain:** Bookmark, source profile, folder/tree, category, tag, sync state, import result, and merge rules.
- **Application:** Use cases for sync, bookmark management, search, import/export, duplicate review/merge, configuration, and health checks. Define small ports where external boundaries need replacement, such as bookmark sources and repositories.
- **Infrastructure:** Chrome profile discovery and snapshot parsing; SQLite connection, migrations, repositories, and FTS5 maintenance; JSON/CSV readers and writers.
- **Commands/UI boundary:** Tauri command handlers validate requests and call application use cases. They should not contain business or SQL logic.
- **UI:** Feature-oriented React screens and components. The UI calls typed application commands and displays progress, warnings, and errors.

Introduce interfaces for real replacement points, not every internal function. Keep the initial implementation concrete enough to remain easy to navigate.

## 2. Project Structure

```text
mind-mark/
  src/                              # React + TypeScript UI
    app/                            # App bootstrap, routing, global state
    features/
      bookmarks/                    # Tree/list, detail, editing
      search/                       # Query, filters, result views
      profiles/                     # Chrome sources and app scopes
      categories/
      tags/
      imports/                      # Import/export flows and reports
      health/                       # Database/index checks and repair actions
    shared/                         # Reusable UI, typed command client
  src-tauri/
    src/
      domain/                       # Entities, value types, invariants
      application/                  # Use cases and ports
      infrastructure/
        chrome/                     # Profile discovery, snapshot parser
        sqlite/                     # Connection, repositories, FTS5
        file_formats/               # JSON and CSV adapters
      commands/                     # Tauri command handlers
      lib.rs
    migrations/                     # Versioned SQL migrations
    tests/                          # Rust integration tests and fixtures
  tests/
    fixtures/                       # Sanitized bookmark and import samples
  docs/                             # Format contracts and architecture decisions
```

Keep unit tests near Rust modules where useful and integration tests at the application/SQLite boundary. Adjust names to the scaffolding conventions after the stack is confirmed.

## 3. Data Model

Use stable internal IDs and explicit source provenance. A starting relational model:

- **app_profiles:** Mind Mark scopes/workspaces. Keep distinct from browser profiles, even if initial onboarding maps one Chrome profile to one app profile.
- **bookmark_sources:** Source type, source profile identity, display name, last successful sync, and status.
- **bookmarks:** Internal ID, app profile ID, title, original URL, normalized URL used for matching, description/notes if supported, timestamps, and archive state if needed.
- **source_bookmarks:** Bookmark ID, source ID, browser/source bookmark ID, observed source metadata, and last-seen sync. Enforce uniqueness for `(source_id, source_bookmark_id)`.
- **folders:** Source/app ownership, parent folder ID, source folder ID where available, name, and stable ordering. Preserve imported hierarchy without using it as a category system.
- **bookmark_categories:** User-defined categories and bookmark-category association.
- **tags / bookmark_tags:** User-managed tags and many-to-many bookmark association.
- **sync_runs / import_runs:** Start/end time, source, status, counts, warnings, and recoverable error summary.
- **settings:** Versioned application preferences; the MVP has no service credentials to store.

Use foreign keys and indexes for profile, parent folder, source identity, normalized URL, and common filters. Keep canonical bookmark data in ordinary tables; search indexes and future embeddings are derived/rebuildable data.

### Data Semantics

- Chrome folders represent source hierarchy; app categories and tags represent user classification.
- Multiple browser sources may eventually refer to the same logical bookmark. Preserve source relationships rather than collapsing provenance.
- Store the original URL unchanged. Normalize only for matching/search policy; avoid silently stripping meaningful query parameters or rewriting user data.
- Source deletion is not app deletion. A missing item in a Chrome snapshot should be marked unseen/stale or reported; any deletion/archive policy must be explicit and reversible.
- Duplicate matching starts with exact normalized-URL candidates. Similar title/domain candidates are suggestions only. Require user confirmation for merge and preserve source IDs, notes, tags, categories, and folder context according to an explicit merge preview.

## 4. Chrome Intake and Sync

### Initial Flow

1. Discover likely Chrome profile directories using platform-specific code isolated in the Chrome adapter; allow manual profile selection.
2. Read the profile's bookmark data as a snapshot. Never mutate Chrome files or call Google account APIs. Chrome remains responsible for synchronizing its local profile with the user's account.
3. Parse and validate the snapshot before writing. Report malformed entries and unsupported node types without discarding valid data.
4. Preview source profile, folders, and bookmark counts; let the user choose the target Mind Mark profile.
5. Upsert records in a transaction using `(source_id, source_bookmark_id)` for idempotency. Update observed fields and last-seen state without duplicating records.
6. Record a sync run with counts, warnings, and result. On failure, roll back the relevant transaction and retain a useful error for the UI.
7. Report added, updated, unchanged, and missing/stale items. Do not automatically remove app records absent from the snapshot.

Start with user-triggered sync. Add scheduled/background sync only after snapshot consistency, repeatability, cancellation, and recovery are tested. Verify Chrome file behavior on supported Windows installations before relying on read-while-running assumptions; use a temporary copied snapshot when appropriate.

## 5. SQLite, Indexing, and Concurrency

- Use versioned migrations from the first schema; avoid ad-hoc runtime schema creation.
- Enable foreign keys, WAL mode, and a bounded busy timeout. Verify settings at initialization and surface failures through health checks.
- Keep write transactions short. Serialize sync/import/merge mutations through one application-level writer queue; allow independent reads where the SQLite binding permits it.
- Do not hold a database transaction while reading/parsing a large input file or waiting for UI interaction.
- Use FTS5 for title, URL, folder path, and selected descriptive fields. Maintain it transactionally or through a documented rebuildable strategy; canonical tables remain authoritative.
- Provide an index health check and rebuild operation. Test interruption and reopening to ensure committed data and derived search state recover consistently.
- SQLite and FTS5 are a suitable starting point for several thousand bookmarks. Benchmark larger fixtures before adding caching or changing databases.

## 6. Search and Future Vectorization

MVP search uses FTS5 for text matching, with structured SQL filters for profile, folder, category, and tags. Define ranking and tokenization behavior in tests.

For a later semantic-search phase, add an embedding-generation interface and vector-store boundary only when scheduled. Track embedding model/version so data can be regenerated. Keep embedding generation optional and failure-tolerant; bookmark management and text search must work offline without a model. Evaluate a local model before a hosted service because bookmark data can be sensitive. Do not let a specific embedding model or vector database become part of the domain model.

## 7. Import, Export, Deduplication, and Health

### JSON and CSV

- Publish a versioned JSON format that can represent bookmarks, hierarchy, categories, tags, and provenance where appropriate.
- Publish a CSV format and explicit column mapping for external CSV files. CSV cannot naturally represent arbitrary nested trees; document how folder paths and multi-valued tags are encoded.
- Validate before mutation and show a preview with accepted, skipped, and invalid rows.
- Apply changes transactionally, using bounded batches if needed; record import runs and define retry behavior.
- Export a consistent snapshot and test JSON/CSV round trips. Do not claim arbitrary third-party CSV compatibility without a mapping.

### Duplicate Review and Merge

- Present candidate groups and the matching reason.
- Distinguish exact URL duplicates from fuzzy suggestions.
- Show which fields, tags, categories, folders, and sources will be preserved before merge.
- Make merge reversible where feasible, or preserve an audit/redirect record so source references are not orphaned.
- Never perform silent destructive deduplication.

### Health Checks

Include database open/schema version checks, foreign-key integrity, FTS5 consistency/rebuild availability, stale/interrupted run detection, and backup location/write checks if backup is included. Distinguish warnings from repairable errors and avoid destructive automatic repair.

## 8. Configuration and Errors

Keep user preferences (selected profiles, sync behavior, search options, import defaults) in versioned app settings with safe defaults and validation. Store the database under the platform's application-data directory. Make its path configurable only if relocation and backup behavior are implemented safely.

Use structured errors at application boundaries. UI-facing results should distinguish validation errors, access/permission issues, malformed source data, database contention, cancellation, and internal failures. Logs must not dump full bookmark URLs or titles by default; bookmark data is sensitive.

## 9. Delivery Sequence and Acceptance Checks

### Phase 1: Foundation

- Confirm desktop stack and supported Windows versions.
- Scaffold app shell and typed Tauri command boundary.
- Add domain types, SQLite initialization, first migrations, and test setup.
- **Accept when:** fresh database creation and upgrade paths pass tests; app can open and report database status.

### Phase 2: Chrome Profile Intake

- Add profile discovery/manual selection, snapshot parsing, preview, and transactional idempotent import.
- Add sanitized fixtures with nested folders, separators, malformed nodes, duplicate IDs, and multiple profiles.
- **Accept when:** second sync creates no duplicates; Chrome files remain untouched; profile/tree relationships survive import; invalid records are reported.

### Phase 3: Bookmark Management

- Add tree/list navigation, bookmark detail/editing, categories, tags, and source provenance display.
- **Accept when:** edits do not corrupt source identity and category/tag changes do not alter imported folder hierarchy.

### Phase 4: Search and Filters

- Add FTS5 indexing, search UI, structured filters, and index health/rebuild.
- **Accept when:** search/filter tests pass after bookmark changes and a rebuild produces equivalent results.

### Phase 5: Portability and Merge

- Add documented JSON/CSV formats, preview/validation, export, duplicate candidates, and reviewed merge.
- **Accept when:** supported-format round trips preserve expected fields; cancelled/failed imports do not leave partial data; merge preserves provenance and associations.

### Phase 6: Reliability

- Add progress, cancellation, interruption recovery, backups/restore if prioritized, and operational health checks.
- **Accept when:** concurrent reads with serialized writes do not lose data; interrupted jobs have clear state; restore and integrity procedures are tested.

### Phase 7: Extensions

- Reassess scheduled sync, semantic search, another browser/platform, and cloud storage independently.
- **Accept when:** each extension has a concrete user need, privacy review, migration plan, and performance evaluation before implementation.

## 10. Risks and Open Decisions

- Validate Chrome profile paths, profile naming, and read behavior while Chrome is open on supported Windows configurations.
- Decide whether app profiles are separate user workspaces or initially a one-to-one mapping to Chrome profiles.
- Define source-change policy for edits and missing bookmarks; initial sync must not silently delete.
- Approve URL normalization and duplicate criteria with examples, including tracking parameters and fragments.
- Define JSON schema version and CSV column mapping/encoding before import/export implementation.
- Confirm whether bookmark editing in Mind Mark is in MVP; edits remain local and are not written back to Chrome.
- Confirm UI accessibility, keyboard navigation, and minimum Windows version before scaffolding.
- Revisit performance targets with real collection sizes; requirements state multiple thousands but no maximum or response-time target.
