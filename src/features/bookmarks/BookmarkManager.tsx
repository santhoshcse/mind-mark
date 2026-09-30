import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Archive, ChevronDown, ChevronRight, CirclePlus, Folder, Layers2, Pencil, Plus, Tag, X } from "lucide-react";

type AppProfile = { id: string; name: string };
type BookmarkFolder = { id: string; parentId: string | null; name: string; position: number };
type Bookmark = {
  id: string;
  appProfileId: string;
  folderId: string | null;
  folderName: string | null;
  title: string;
  url: string;
  createdAt: string;
  updatedAt: string;
  position: number;
  tags: string[];
  categories: string[];
  sourceProfileName: string | null;
};
type Draft = { id?: string; title: string; url: string; folderId: string; tags: string; categories: string };

type Props = { reloadKey: number; preferredProfileId?: string };

const emptyDraft: Draft = { title: "", url: "", folderId: "", tags: "", categories: "" };

function splitValues(value: string): string[] {
  return [...new Set(value.split(",").map((item) => item.trim()).filter(Boolean))];
}

export default function BookmarkManager({ reloadKey, preferredProfileId }: Props) {
  const [profiles, setProfiles] = useState<AppProfile[]>([]);
  const [folders, setFolders] = useState<BookmarkFolder[]>([]);
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
  const [activeProfileId, setActiveProfileId] = useState("personal");
  const [selectedFolderId, setSelectedFolderId] = useState("");
  const [draft, setDraft] = useState<Draft | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [error, setError] = useState("");
  const [isLoading, setIsLoading] = useState(true);

  async function loadProfiles() {
    const nextProfiles = await invoke<AppProfile[]>("list_app_profiles");
    setProfiles(nextProfiles);
    const nextProfileId = preferredProfileId && nextProfiles.some((profile) => profile.id === preferredProfileId)
      ? preferredProfileId
      : nextProfiles.some((profile) => profile.id === activeProfileId)
        ? activeProfileId
        : nextProfiles[0]?.id ?? "";
    setActiveProfileId(nextProfileId);
    return nextProfileId;
  }

  async function loadLibrary(profileId = activeProfileId, folderId = selectedFolderId) {
    if (!profileId) return;
    const [nextFolders, nextBookmarks] = await Promise.all([
      invoke<BookmarkFolder[]>("list_folders", { appProfileId: profileId }),
      invoke<Bookmark[]>("list_bookmarks", {
        appProfileId: profileId,
        folderId: folderId || null,
      }),
    ]);
    setFolders(nextFolders);
    setBookmarks(nextBookmarks);
  }

  async function refresh() {
    setIsLoading(true);
    setError("");
    try {
      const profileId = await loadProfiles();
      setSelectedFolderId("");
      await loadLibrary(profileId, "");
    } catch (caught) {
      setError(String(caught));
    } finally {
      setIsLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, [reloadKey]);

  async function changeProfile(profileId: string) {
    setActiveProfileId(profileId);
    setSelectedFolderId("");
    setIsLoading(true);
    setError("");
    try {
      await loadLibrary(profileId, "");
    } catch (caught) {
      setError(String(caught));
    } finally {
      setIsLoading(false);
    }
  }

  async function chooseFolder(folderId: string) {
    setSelectedFolderId(folderId);
    setIsLoading(true);
    setError("");
    try {
      await loadLibrary(activeProfileId, folderId);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setIsLoading(false);
    }
  }

  async function addProfile() {
    const name = window.prompt("Name this Mind Mark profile");
    if (!name?.trim()) return;
    setError("");
    try {
      const profile = await invoke<AppProfile>("create_app_profile", { name });
      setProfiles((current) => [...current, profile].sort((left, right) => left.name.localeCompare(right.name)));
      setActiveProfileId(profile.id);
      setSelectedFolderId("");
      setBookmarks([]);
      setFolders([]);
    } catch (caught) {
      setError(String(caught));
    }
  }

  function editBookmark(bookmark: Bookmark) {
    setDraft({
      id: bookmark.id,
      title: bookmark.title,
      url: bookmark.url,
      folderId: bookmark.folderId ?? "",
      tags: bookmark.tags.join(", "),
      categories: bookmark.categories.join(", "),
    });
  }

  async function saveBookmark(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!draft) return;
    setError("");
    const input = {
      title: draft.title,
      url: draft.url,
      folderId: draft.folderId || null,
      tags: splitValues(draft.tags),
      categories: splitValues(draft.categories),
    };
    try {
      if (draft.id) {
        await invoke("update_bookmark", { input: { id: draft.id, ...input } });
      } else {
        await invoke("create_bookmark", { input: { appProfileId: activeProfileId, ...input } });
      }
      setDraft(null);
      await loadLibrary();
    } catch (caught) {
      setError(String(caught));
    }
  }

  async function archiveBookmark(bookmark: Bookmark) {
    if (!window.confirm(`Archive “${bookmark.title}” from this library?`)) return;
    setError("");
    try {
      await invoke("archive_bookmark", { id: bookmark.id });
      await loadLibrary();
    } catch (caught) {
      setError(String(caught));
    }
  }

  function renderFolder(folder: BookmarkFolder, depth = 0): ReactNode {
    const children = folders.filter((candidate) => candidate.parentId === folder.id);
    const isExpanded = expandedFolders.has(folder.id);
    return (
      <div className="tree-branch" key={folder.id}>
        <div className={`tree-row ${selectedFolderId === folder.id ? "tree-row-active" : ""}`} style={{ paddingLeft: 9 + depth * 13 }}>
          {children.length > 0 ? (
            <button
              className="tree-disclosure"
              type="button"
              aria-label={`${isExpanded ? "Collapse" : "Expand"} ${folder.name}`}
              onClick={() => setExpandedFolders((current) => {
                const next = new Set(current);
                if (next.has(folder.id)) next.delete(folder.id);
                else next.add(folder.id);
                return next;
              })}
            >
              {isExpanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
            </button>
          ) : <span className="tree-disclosure-spacer" />}
          <button className="tree-folder" type="button" onClick={() => void chooseFolder(folder.id)}>
            <Folder size={14} />
            <span>{folder.name}</span>
          </button>
        </div>
        {isExpanded && children.map((child) => renderFolder(child, depth + 1))}
      </div>
    );
  }

  const activeProfileName = profiles.find((profile) => profile.id === activeProfileId)?.name ?? "Profile";
  const rootFolders = folders.filter((folder) => !folder.parentId);

  return (
    <section className="library-manager" aria-label="Bookmark collection">
      <aside className="folder-sidebar">
        <div className="folder-sidebar-heading">
          <div className="eyebrow">WORKSPACE</div>
          <button className="icon-button" type="button" title="Create profile" aria-label="Create profile" onClick={() => void addProfile()}>
            <CirclePlus size={16} />
          </button>
        </div>
        <label className="profile-picker-label" htmlFor="app-profile">PROFILE</label>
        <select id="app-profile" className="app-profile-select" value={activeProfileId} onChange={(event) => void changeProfile(event.target.value)}>
          {profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
        </select>
        <button className={`all-bookmarks ${selectedFolderId ? "" : "all-bookmarks-active"}`} type="button" onClick={() => void chooseFolder("")}>
          <Layers2 size={14} />
          <span>All bookmarks</span>
          <span className="tree-count">{bookmarks.length}</span>
        </button>
        <div className="folder-tree-heading">FOLDERS</div>
        <div className="folder-tree">
          {rootFolders.map((folder) => renderFolder(folder))}
          {!isLoading && folders.length === 0 && <div className="tree-empty">Folders appear after Chrome sync.</div>}
        </div>
      </aside>

      <div className="bookmark-list-panel">
        <div className="bookmark-list-heading">
          <div>
            <div className="eyebrow">{activeProfileName.toUpperCase()}</div>
            <h2>{selectedFolderId ? folders.find((folder) => folder.id === selectedFolderId)?.name ?? "Folder" : "All bookmarks"}</h2>
            <p>{bookmarks.length} {bookmarks.length === 1 ? "bookmark" : "bookmarks"}</p>
          </div>
          <button className="button button-primary" type="button" onClick={() => setDraft({ ...emptyDraft })}>
            <Plus size={15} /> Add bookmark
          </button>
        </div>

        {error && <div className="feedback feedback-error manager-error" role="alert">{error}</div>}

        {isLoading ? (
          <div className="manager-empty">Loading bookmarks...</div>
        ) : bookmarks.length === 0 ? (
          <div className="manager-empty">
            <div className="empty-icon"><Folder size={22} /></div>
            <h3>{folders.length === 0 ? "No bookmarks in this profile yet" : "This folder is empty"}</h3>
            <p>{folders.length === 0 ? "Sync a Chrome profile or add a bookmark to begin." : "Choose another folder or add a bookmark here."}</p>
          </div>
        ) : (
          <div className="bookmark-rows">
            {bookmarks.map((bookmark) => (
              <article className="bookmark-row" key={bookmark.id}>
                <div className="bookmark-favicon">{bookmark.title.trim().charAt(0).toUpperCase() || "B"}</div>
                <button className="bookmark-row-main" type="button" onClick={() => editBookmark(bookmark)}>
                  <span className="bookmark-title">{bookmark.title || bookmark.url}</span>
                  <span className="bookmark-url">{bookmark.url}</span>
                  <span className="bookmark-classifications">
                    {bookmark.folderName && <span className="classification folder-chip"><Folder size={11} />{bookmark.folderName}</span>}
                    {bookmark.categories.map((category) => <span className="classification category-chip" key={category}><Layers2 size={11} />{category}</span>)}
                    {bookmark.tags.map((tag) => <span className="classification tag-chip" key={tag}><Tag size={11} />{tag}</span>)}
                    {bookmark.sourceProfileName && <span className="bookmark-source-label">Chrome · {bookmark.sourceProfileName}</span>}
                  </span>
                </button>
                <div className="bookmark-row-actions">
                  <button className="icon-button" type="button" title="Edit bookmark" aria-label={`Edit ${bookmark.title}`} onClick={() => editBookmark(bookmark)}><Pencil size={14} /></button>
                  <button className="icon-button icon-button-danger" type="button" title="Archive bookmark" aria-label={`Archive ${bookmark.title}`} onClick={() => void archiveBookmark(bookmark)}><Archive size={14} /></button>
                </div>
              </article>
            ))}
          </div>
        )}
      </div>

      {draft && (
        <div className="dialog-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setDraft(null); }}>
          <section className="bookmark-dialog" role="dialog" aria-modal="true" aria-labelledby="bookmark-dialog-title">
            <header className="dialog-heading">
              <div><div className="eyebrow">{draft.id ? "EDIT ITEM" : "NEW ITEM"}</div><h2 id="bookmark-dialog-title">{draft.id ? "Edit bookmark" : "Add bookmark"}</h2></div>
              <button className="icon-button" type="button" aria-label="Close" onClick={() => setDraft(null)}><X size={17} /></button>
            </header>
            <form onSubmit={(event) => void saveBookmark(event)}>
              <label className="form-field"><span>Title</span><input required maxLength={500} autoFocus value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.target.value })} /></label>
              <label className="form-field"><span>URL</span><input required type="url" value={draft.url} onChange={(event) => setDraft({ ...draft, url: event.target.value })} /></label>
              <label className="form-field"><span>Folder</span>
                <select value={draft.folderId} onChange={(event) => setDraft({ ...draft, folderId: event.target.value })}>
                  <option value="">No folder</option>
                  {folders.map((folder) => <option key={folder.id} value={folder.id}>{folder.name}</option>)}
                </select>
              </label>
              <label className="form-field"><span>Categories <small>Comma separated</small></span><input value={draft.categories} onChange={(event) => setDraft({ ...draft, categories: event.target.value })} placeholder="Reference, Read later" /></label>
              <label className="form-field"><span>Tags <small>Comma separated</small></span><input value={draft.tags} onChange={(event) => setDraft({ ...draft, tags: event.target.value })} placeholder="work, rust, research" /></label>
              <footer className="dialog-actions">
                <button className="button button-outline" type="button" onClick={() => setDraft(null)}>Cancel</button>
                <button className="button button-primary" type="submit"><Plus size={14} />Save bookmark</button>
              </footer>
            </form>
          </section>
        </div>
      )}
    </section>
  );
}
