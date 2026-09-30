import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AlertCircle, ArrowDownToLine, Bookmark, Check, ChevronDown, FolderTree, HardDrive, RefreshCw, Search } from "lucide-react";
import BookmarkManager from "./features/bookmarks/BookmarkManager";
import "./App.css";

type ChromeProfile = {
  id: string;
  name: string;
  path: string;
};

type SyncReport = {
  profileName: string;
  added: number;
  updated: number;
  unchanged: number;
  folders: number;
  skipped: number;
  missing: number;
  warnings: string[];
};

function App() {
  const [profiles, setProfiles] = useState<ChromeProfile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState("");
  const [report, setReport] = useState<SyncReport | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [isSyncing, setIsSyncing] = useState(false);
  const [error, setError] = useState("");

  async function refreshProfiles() {
    setIsLoading(true);
    setError("");
    try {
      const discovered = await invoke<ChromeProfile[]>("list_chrome_profiles");
      setProfiles(discovered);
      setSelectedProfileId((current) =>
        discovered.some((profile) => profile.id === current)
          ? current
          : discovered[0]?.id ?? "",
      );
    } catch (caught) {
      setError(String(caught));
    } finally {
      setIsLoading(false);
    }
  }

  async function syncSelectedProfile() {
    if (!selectedProfileId) return;

    setIsSyncing(true);
    setError("");
    setReport(null);
    try {
      const result = await invoke<SyncReport>("sync_chrome_profile", {
        profileId: selectedProfileId,
      });
      setReport(result);
      setReloadKey((current) => current + 1);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setIsSyncing(false);
    }
  }

  useEffect(() => {
    void refreshProfiles();
  }, []);

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <a className="brand" href="#library" aria-label="Mind Mark home">
          <span className="brand-mark"><Bookmark size={19} strokeWidth={2.4} /></span>
          <span>mind mark</span>
        </a>

        <div className="sidebar-label">LIBRARY</div>
        <button className="nav-item nav-item-active" type="button">
          <Bookmark size={17} />
          <span>Bookmarks</span>
          <span className="nav-count">{report ? report.added + report.updated + report.unchanged : "0"}</span>
        </button>
        <button className="nav-item" type="button" onClick={() => document.getElementById("sources")?.scrollIntoView({ behavior: "smooth" })}>
          <HardDrive size={17} />
          <span>Sources</span>
          <span className="nav-count">{profiles.length}</span>
        </button>

        <div className="sidebar-bottom">
          <span className="local-indicator" />
          <span>Stored on this device</span>
        </div>
      </aside>

      <section className="workspace" id="library">
        <header className="topbar">
          <div className="breadcrumbs"><span>Library</span><span className="crumb-divider">/</span><strong>Bookmarks</strong></div>
          <div className="topbar-actions">
            <label className="search-box">
              <Search size={16} />
              <input aria-label="Search bookmarks" placeholder="Search bookmarks" disabled />
              <kbd>Ctrl K</kbd>
            </label>
            <span className="avatar" aria-label="Local profile">M</span>
          </div>
        </header>

        <div className="content">
          <div className="page-heading">
            <div>
              <div className="eyebrow">YOUR COLLECTION</div>
              <h1>Bookmarks</h1>
              <p className="page-subtitle">A local library, ready for your Chrome collection.</p>
            </div>
            <button className="button button-primary" type="button" onClick={() => document.getElementById("sources")?.scrollIntoView({ behavior: "smooth" })}>
              <ArrowDownToLine size={16} />
              Import from Chrome
            </button>
          </div>

          <section className="collection-toolbar" aria-label="Bookmark list controls">
            <div className="view-tabs"><button className="view-tab view-tab-active" type="button"><FolderTree size={15} />Tree</button></div>
            <div className="toolbar-meta"><span>{report ? `${report.added + report.updated + report.unchanged} synced` : "No bookmarks yet"}</span><ChevronDown size={14} /></div>
          </section>

          <BookmarkManager
            reloadKey={reloadKey}
            preferredProfileId={report ? `chrome-profile:${selectedProfileId}` : undefined}
          />

          <section className="sources-section" id="sources">
            <div className="section-heading">
              <div>
                <div className="eyebrow">CONNECTIONS</div>
                <h2>Chrome profiles</h2>
              </div>
              <button className="button button-quiet" type="button" onClick={() => void refreshProfiles()} disabled={isLoading || isSyncing}>
                <RefreshCw size={15} className={isLoading ? "spin" : ""} />
                Refresh
              </button>
            </div>

            <div className="source-panel">
              <div className="source-panel-top">
                <div className="source-title-wrap">
                  <div className="chrome-mark">C</div>
                  <div><h3>Google Chrome</h3><p>Read-only bookmark source</p></div>
                </div>
                <span className="source-status"><span className="status-dot" />LOCAL</span>
              </div>

              {isLoading ? (
                <div className="source-state">Looking for Chrome profiles...</div>
              ) : profiles.length === 0 ? (
                <div className="source-state">No Chrome profiles with bookmark data were found on this device.</div>
              ) : (
                <div className="source-controls">
                  <label className="profile-select-label" htmlFor="chrome-profile">PROFILE</label>
                  <select id="chrome-profile" value={selectedProfileId} onChange={(event) => setSelectedProfileId(event.target.value)}>
                    {profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
                  </select>
                  <button className="button button-primary sync-button" type="button" onClick={() => void syncSelectedProfile()} disabled={isSyncing || !selectedProfileId}>
                    <RefreshCw size={15} className={isSyncing ? "spin" : ""} />
                    {isSyncing ? "Syncing" : "Sync bookmarks"}
                  </button>
                </div>
              )}

              {error && <div className="feedback feedback-error" role="alert"><AlertCircle size={16} /><span>{error}</span></div>}
              {report && <div className="feedback feedback-success" role="status"><Check size={16} /><span>{report.profileName}: {report.added} added, {report.updated} updated, {report.unchanged} unchanged, {report.missing} missing (kept), {report.skipped} skipped.</span></div>}
              {report?.warnings.map((warning) => <div className="warning-text" key={warning}>{warning}</div>)}
              <div className="source-footnote">Mind Mark reads Chrome snapshots and never edits Chrome bookmarks.</div>
            </div>
          </section>
        </div>
      </section>
    </main>
  );
}

export default App;
