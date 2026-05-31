import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";


interface AccountRow {
  username: string;
  password: string;
  role?: string;
}


interface BuildFileEntry {
  name: string;
  path: string;
  url: string;
  sha1: string;
  size: number;
  enabled: boolean;
}

interface BuildManifest {
  name: string;
  minecraft_version: string;
  loader: string;
  loader_version: string;
  mods: BuildFileEntry[];
  server_ip?: string;
  discord_url?: string;
}

interface UploadProgress {
  done: number;
  total: number;
  current: string;
  errors: string[];
  finished: boolean;
}

interface GitHubTreeEntry {
  path: string;
  mode: string;
  type: string;
  sha: string;
  size?: number;
  url: string;
}

interface FileTreeNode extends GitHubTreeEntry {
  name: string;
  children: FileTreeNode[];
}

interface Props {
  username: string;
  isOwner: boolean;
  onDiscordUrlChange?: (url: string) => void;
}


const ADMIN_NAME = "DarkSpark00";
const BUILD_NAMES = ["danganverse"];
const LOADERS = ["vanilla", "forge", "fabric", "neoforge", "optifine"];

const formatSize = (size: number) => `${(size / 1024 / 1024).toFixed(1)} МБ`;

export default function AdminPanel({ username, isOwner, onDiscordUrlChange }: Props) {

  const [activeTab, setActiveTab] = useState<"accounts" | "builds">("accounts");
  const [accounts, setAccounts] = useState<AccountRow[]>([]);
  const [githubToken, setGithubToken] = useState("");
  const [message, setMessage] = useState("");
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);


  const [saving, setSaving] = useState(false);
  const [showPasswords, setShowPasswords] = useState<Record<string, boolean>>({});
  const [activeBuild, setActiveBuild] = useState("danganverse");
  const [manifest, setManifest] = useState<BuildManifest | null>(null);
  const [uploadingMod, setUploadingMod] = useState(false);
  const [uploadingBuild, setUploadingBuild] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress | null>(null);
  const [modSearch, setModSearch] = useState("");
  const [availableVersions, setAvailableVersions] = useState<string[]>([]);
  const [downloadDir, setDownloadDir] = useState("");
  const lastDropKeyRef = useRef("");
  const uploadPollRef = useRef<number | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);

  const [repoTree, setRepoTree] = useState<GitHubTreeEntry[]>([]);
  const [builtTree, setBuiltTree] = useState<FileTreeNode[]>([]);
  const [treeLoading, setTreeLoading] = useState(false);
  const [treeError, setTreeError] = useState("");
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set(["mods"]));
  const [deletingPath, setDeletingPath] = useState<string | null>(null);
  const [showModList, setShowModList] = useState(false);


  useEffect(() => {


    load();
    loadToken();
    loadVersions();
    loadDownloadDir();
  }, []);

  const loadVersions = async () => {
    try {
      const resp = await invoke<any[]>("get_mc_versions");
      const releaseVersions = resp
        .filter(v => (v.version_type ?? v.type) === "release")
        .map(v => v.id);

      setAvailableVersions(releaseVersions);
    } catch {
      setAvailableVersions([]);
    }
  };

  const loadDownloadDir = async () => {
    try {
      setDownloadDir(await invoke<string>("get_build_download_dir"));
    } catch (e) {
        // error ignored
      }
  };


  useEffect(() => {
    if (isOwner && githubToken.trim()) loadManifest(activeBuild);
  }, [activeBuild, githubToken, isOwner]);

  useEffect(() => {
    if (isOwner && githubToken.trim() && manifest && activeTab === "builds") {
      setRepoTree([]);
      setBuiltTree([]);
      setTreeError("");
    }
  }, [activeBuild]);

  useEffect(() => {
    if (!isOwner || activeTab !== "builds" || !manifest) return;
    let unlisten: (() => void) | undefined;
    getCurrentWindow().onDragDropEvent(async (event) => {
      if (event.payload.type !== "drop") return;
      await handleDroppedPaths(event.payload.paths);
    }).then(fn => { unlisten = fn; }).catch(error => notify(`Drag & drop недоступен: ${String(error)}`));
    return () => { if (unlisten) unlisten(); };
  }, [isOwner, activeTab, manifest, activeBuild, githubToken]);

  useEffect(() => {
    if (!isOwner || activeTab !== "builds") return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Control" || event.repeat) return;
      const panel = panelRef.current;
      if (!panel) return;
      const nearBottom = panel.scrollTop + panel.clientHeight >= panel.scrollHeight - 24;
      panel.scrollTo({ top: nearBottom ? 0 : panel.scrollHeight, behavior: "smooth" });
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isOwner, activeTab]);


  const notify = (text: string) => {
    setMessage(text);
    const id = Date.now() + Math.floor(Math.random() * 1000);
    setToasts(prev => [...prev, { id, text }].slice(-4));
    window.setTimeout(() => setToasts(prev => prev.filter(item => item.id !== id)), 4500);
  };


  const load = async () => {
    try {
      const list = await invoke<AccountRow[]>("get_admin_accounts", { currentUsername: username });
      setAccounts(list);
    } catch (e) {
      setMessage(String(e));
    }
  };

  const loadToken = async () => {
    try {
      const token = await invoke<string>("get_admin_token", { currentUsername: username });
      setGithubToken(token);
    } catch {

    }
  };

  const loadManifest = async (build: string) => {
    try {
      const data = await invoke<BuildManifest>("get_build_manifest", { build, githubToken });
      setManifest(data);
    } catch (e) {
      setMessage(String(e));
      setManifest(null);
    }
  };

  const saveToken = async (token: string) => {
    setGithubToken(token);
    try {
      await invoke("save_admin_token", { currentUsername: username, githubToken: token });
    } catch {

    }
  };

  const updatePassword = (index: number, password: string) => {
    setAccounts(prev => prev.map((row, i) => i === index ? { ...row, password } : row));
  };

  const deleteAccount = (account: AccountRow) => {
    if (account.username.toLowerCase() === ADMIN_NAME.toLowerCase()) {
      setMessage("Нельзя удалить DarkSpark00");
      return;
    }
    const ok = window.confirm(`Удалить игрока ${account.username}? Это применится после commit.`);
    if (!ok) return;
    setAccounts(prev => prev.filter(a => a.username !== account.username));
  };

  const commitChanges = async () => {
    setSaving(true);
    setMessage("Шифрую файл и отправляю commit на GitHub...");
    try {
      const result = await invoke<string>("commit_admin_accounts", {
        currentUsername: username,
        githubToken,
        accounts,
      });
      setMessage(result);
    } catch (e) {
      setMessage(String(e));
    } finally {
      setSaving(false);
    }
  };

  const updateManifest = (patch: Partial<BuildManifest>) => {
    setManifest(prev => prev ? { ...prev, ...patch } : prev);
  };

  const updateMod = (index: number, patch: Partial<BuildFileEntry>) => {
    setManifest(prev => prev ? {
      ...prev,
      mods: prev.mods.map((mod, i) => i === index ? { ...mod, ...patch } : mod),
    } : prev);
  };

  const deleteMod = async (mod: BuildFileEntry) => {
    const treeEntry = repoTree.find(e => e.path === mod.path && e.type === "blob");
    const willDeleteGithub = !!treeEntry;
    const ok = window.confirm(
      `Удалить мод ${mod.name}?\n\n` +
      (willDeleteGithub
        ? "Файл будет удалён из GitHub и из списка манифеста."
        : "Файл не найден в дереве GitHub — удаляется только из списка манифеста.")
    );
    if (!ok) return;
    if (willDeleteGithub && treeEntry) {
      setDeletingPath(treeEntry.path);
      try {
        await invoke("delete_build_file", { build: activeBuild, githubToken, filePath: treeEntry.path, sha: treeEntry.sha });
        const newTree = repoTree.filter(e => e.path !== treeEntry.path);
        setRepoTree(newTree);
        setBuiltTree(buildFileTree(newTree));
      } catch (e) {
        notify(`Ошибка удаления с GitHub: ${String(e)}`);
      } finally {
        setDeletingPath(null);
      }
    }
    setManifest(prev => prev ? { ...prev, mods: prev.mods.filter(m => m.name !== mod.name) } : prev);
    notify(`Мод ${mod.name} удалён. Нажмите commit, чтобы сохранить manifest.`);
  };


  const downloadMod = async (mod: BuildFileEntry) => {
    notify(`Скачиваю ${mod.name}...`);

    try {
      const path = await invoke<string>("download_build_mod_file", { modEntry: mod });
      notify(`Мод сохранён: ${path}`);

    } catch (e) {
      setMessage(String(e));
    }
  };

  const downloadBuild = async () => {
    if (!manifest) return;
    notify(`Скачиваю сборку ${activeBuild}...`);

    try {
      const result = await invoke<string>("download_build_bundle", { build: activeBuild, manifest });
      setMessage(result);
    } catch (e) {
      setMessage(String(e));
    }
  };

  const chooseDownloadDir = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false, title: "Выберите папку сохранения" });
      if (typeof selected === "string") {
        await invoke("set_build_download_dir", { path: selected });
        setDownloadDir(selected);
        notify(`Папка сохранения: ${selected}`);

      }
    } catch (e) {
      setMessage(String(e));
    }
  };


  const uploadModPath = async (path: string) => {
    if (!manifest || !path) return;
    if (!githubToken.trim()) {
      notify("Введите GitHub token перед загрузкой мода");
      return;
    }
    setUploadingMod(true);
    notify(`Загружаю мод ${path}...`);
    try {

      const entry = await invoke<BuildFileEntry>("upload_build_mod", {
        build: activeBuild,
        githubToken,
        filePath: path,
        targetName: null,
      });
      setManifest(prev => prev ? {
        ...prev,
        mods: [...prev.mods.filter(m => m.name !== entry.name), entry],
      } : prev);
      if (repoTree.length > 0) loadRepoTree();
      notify(`Мод ${entry.name} загружен. Нажмите «Сохранить manifest», чтобы он вошёл в сборку.`);

    } catch (e) {
      notify(`Не удалось загрузить мод: ${String(e)}`);
    } finally {
      setUploadingMod(false);
    }
  };

  const handleDroppedPaths = async (paths: string[]) => {
    const key = paths.join("|");
    if (key && key === lastDropKeyRef.current) return;
    lastDropKeyRef.current = key;
    window.setTimeout(() => {
      if (lastDropKeyRef.current === key) lastDropKeyRef.current = "";
    }, 1200);

    const uploadablePaths = paths.filter(path => {
      const lower = path.toLowerCase();
      return lower.endsWith(".jar") || lower.endsWith(".zip");
    });
    if (uploadablePaths.length === 0) {
      notify("Перетащите .jar (мод) или .zip (ресурспак/шейдер), либо нажмите кнопку выбора файла");
      return;
    }
    for (const path of uploadablePaths) {
      await uploadModPath(path);
    }
  };

  const onDropMod = async (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    const files = Array.from(event.dataTransfer.files);
    const paths = files.map(file => (file as any).path || (file as any).webkitRelativePath).filter(Boolean);
    if (paths.length === 0) {
      notify("WebView не отдал путь файла. Нажмите «Выбрать .jar / .zip» или перетащите файл в окно лаунчера.");
      return;
    }
    await handleDroppedPaths(paths);
  };


  const chooseModFiles = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ multiple: true, directory: false, filters: [{ name: "Mods & Packs", extensions: ["jar", "zip"] }] });
      const paths = Array.isArray(selected) ? selected : (typeof selected === "string" ? [selected] : []);
      if (paths.length === 0) return;
      for (const path of paths) {
        await uploadModPath(path);
      }
    } catch (e) {
      notify(String(e));
    }
  };

  const buildFileTree = (entries: GitHubTreeEntry[]): FileTreeNode[] => {
    const map = new Map<string, FileTreeNode>();
    for (const e of entries) {
      const name = e.path.includes("/") ? e.path.split("/").pop()! : e.path;
      map.set(e.path, { ...e, name, children: [] });
    }
    const roots: FileTreeNode[] = [];
    for (const [path, node] of map) {
      const idx = path.lastIndexOf("/");
      if (idx === -1) { roots.push(node); }
      else {
        const parent = map.get(path.substring(0, idx));
        if (parent) parent.children.push(node); else roots.push(node);
      }
    }
    const sort = (nodes: FileTreeNode[]) => {
      nodes.sort((a, b) => a.type !== b.type ? (a.type === "tree" ? -1 : 1) : a.name.localeCompare(b.name));
      nodes.forEach(n => n.type === "tree" && sort(n.children));
    };
    sort(roots);
    return roots;
  };

  const countBlobs = (node: FileTreeNode): number => {
    if (node.type === "blob") return 1;
    return node.children.reduce((s, c) => s + countBlobs(c), 0);
  };

  const getFileIcon = (name: string): string => {
    const ext = name.split(".").pop()?.toLowerCase() || "";
    if (ext === "jar") return "🟦";
    if (ext === "zip") return "🗜️";
    if (ext === "png" || ext === "jpg" || ext === "gif") return "🖼️";
    if (["json", "toml", "cfg", "conf", "yaml", "yml", "xml", "ini", "properties"].includes(ext)) return "⚙️";
    if (ext === "txt" || ext === "md") return "📝";
    return "📄";
  };

  const loadRepoTree = async () => {
    if (!githubToken.trim()) { notify("Введите GitHub token"); return; }
    setTreeLoading(true);
    setTreeError("");
    try {
      const tree = await invoke<GitHubTreeEntry[]>("get_build_git_tree", { build: activeBuild, githubToken });
      setRepoTree(tree);
      setBuiltTree(buildFileTree(tree));
      setExpandedPaths(new Set(["mods"]));
    } catch (e) {
      setTreeError(String(e));
    } finally {
      setTreeLoading(false);
    }
  };

  const toggleExpand = (path: string) => {
    setExpandedPaths(prev => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path); else next.add(path);
      return next;
    });
  };

  const deleteFile = async (node: FileTreeNode) => {
    if (!window.confirm(`Удалить файл из GitHub?\n\n${node.path}`)) return;
    setDeletingPath(node.path);
    try {
      await invoke("delete_build_file", { build: activeBuild, githubToken, filePath: node.path, sha: node.sha });
      const newTree = repoTree.filter(e => e.path !== node.path);
      setRepoTree(newTree);
      setBuiltTree(buildFileTree(newTree));
      if (node.path.startsWith("mods/")) {
        setManifest(prev => prev ? { ...prev, mods: prev.mods.filter(m => m.path !== node.path) } : prev);
      }
      notify(`Удалён: ${node.name}`);
    } catch (e) { notify(`Ошибка удаления: ${String(e)}`); }
    finally { setDeletingPath(null); }
  };

  const deleteFolder = async (node: FileTreeNode) => {
    const blobs = repoTree.filter(e => e.type === "blob" && e.path.startsWith(node.path + "/"));
    if (blobs.length === 0) { notify("Папка пуста"); return; }
    if (!window.confirm(`Удалить папку ${node.name}/ и все ${blobs.length} файлов внутри?\n\nЭто необратимо.`)) return;
    setDeletingPath(node.path);
    let deleted = 0;
    for (const blob of blobs) {
      try {
        await invoke("delete_build_file", { build: activeBuild, githubToken, filePath: blob.path, sha: blob.sha });
        deleted++;
      } catch (e) { notify(`Ошибка: ${blob.path}: ${String(e)}`); }
    }
    const remaining = repoTree.filter(e => !e.path.startsWith(node.path + "/") && e.path !== node.path);
    setRepoTree(remaining);
    setBuiltTree(buildFileTree(remaining));
    const deletedPaths = new Set(blobs.map(b => b.path));
    setManifest(prev => prev ? { ...prev, mods: prev.mods.filter(m => !deletedPaths.has(m.path)) } : prev);
    setDeletingPath(null);
    notify(`Удалено ${deleted}/${blobs.length} файлов из ${node.name}/`);
  };

  const renderTreeNode = (node: FileTreeNode, depth: number): React.ReactNode => {
    const isDir = node.type === "tree";
    const isExpanded = expandedPaths.has(node.path);
    const isDeleting = deletingPath === node.path || (deletingPath !== null && node.path.startsWith(deletingPath + "/"));
    return (
      <div key={node.path}>
        <div
          style={{ display: "flex", alignItems: "center", gap: 3, paddingLeft: depth * 16 + 6, paddingRight: 6, minHeight: 28, borderRadius: 4, opacity: isDeleting ? 0.35 : 1, transition: "opacity 0.2s" }}
          className="admin-tree-row"
        >
          {isDir
            ? <button onClick={() => toggleExpand(node.path)} style={{ background: "none", border: "none", cursor: "pointer", padding: "0 2px", fontSize: 10, color: "inherit", width: 14, flexShrink: 0, opacity: 0.6 }}>{isExpanded ? "▼" : "▶"}</button>
            : <span style={{ width: 14, flexShrink: 0, display: "inline-block" }} />}
          <span style={{ fontSize: 13, flexShrink: 0 }}>{isDir ? "📁" : getFileIcon(node.name)}</span>
          <span style={{ flex: 1, fontSize: 12, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", marginLeft: 2 }} title={node.path}>
            {node.name}{isDir && <span style={{ fontSize: 10, opacity: 0.45, marginLeft: 4 }}>({countBlobs(node)})</span>}
          </span>
          {!isDir && node.size !== undefined && <span style={{ fontSize: 10, opacity: 0.45, marginRight: 4, whiteSpace: "nowrap" }}>{formatSize(node.size)}</span>}
          {!isDeleting && (
            <button className="settings-btn danger compact" style={{ fontSize: 10, padding: "2px 6px", flexShrink: 0 }}
              onClick={() => isDir ? deleteFolder(node) : deleteFile(node)}
              disabled={deletingPath !== null}
              title={isDir ? `Удалить папку ${node.name}/` : `Удалить ${node.name}`}>
              ✕
            </button>
          )}
          {isDeleting && <span style={{ fontSize: 10, opacity: 0.5, flexShrink: 0 }}>удаление…</span>}
        </div>
        {isDir && isExpanded && (
          <div>
            {node.children.map(child => renderTreeNode(child, depth + 1))}
            {node.children.length === 0 && <div style={{ paddingLeft: (depth + 1) * 16 + 20, fontSize: 11, opacity: 0.35, padding: `2px 0 2px ${(depth + 1) * 16 + 20}px` }}>(пусто)</div>}
          </div>
        )}
      </div>
    );
  };

  const uploadModpackFolder = async () => {
    if (!githubToken.trim()) { notify("Введите GitHub token перед загрузкой"); return; }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false, title: "Выберите папку модпака (например: danganverse)" });
      if (typeof selected !== "string") return;

      setUploadingBuild(true);
      setUploadProgress({ done: 0, total: 0, current: "Сканирование файлов...", errors: [], finished: false });

      uploadPollRef.current = window.setInterval(async () => {
        try {
          const prog = await invoke<UploadProgress | null>("get_upload_progress");
          if (prog) setUploadProgress(prog);
        } catch {}
      }, 800);

      try {
        const entries = await invoke<BuildFileEntry[]>("upload_modpack_build", {
          build: activeBuild,
          githubToken,
          folderPath: selected,
        });
        setManifest(prev => prev ? { ...prev, mods: entries } : prev);
        notify(`Загружено ${entries.length} файлов. Нажмите «Commit» чтобы сохранить manifest.`);
      } catch (e) {
        notify(`Ошибка загрузки сборки: ${String(e)}`);
      } finally {
        if (uploadPollRef.current !== null) {
          window.clearInterval(uploadPollRef.current);
          uploadPollRef.current = null;
        }
        setUploadingBuild(false);
      }
    } catch (e) {
      notify(String(e));
    }
  };

  const chooseAndUploadZip = async () => {
    if (!githubToken.trim()) { notify("Введите GitHub token перед загрузкой"); return; }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ multiple: false, directory: false, filters: [{ name: "ZIP архив сборки", extensions: ["zip"] }] });
      if (typeof selected !== "string") return;
      setUploadingBuild(true);
      setUploadProgress({ done: 0, total: 0, current: "Извлечение ZIP...", errors: [], finished: false });
      uploadPollRef.current = window.setInterval(async () => {
        try { const prog = await invoke<UploadProgress | null>("get_upload_progress"); if (prog) setUploadProgress(prog); } catch {}
      }, 800);
      try {
        const entries = await invoke<BuildFileEntry[]>("upload_build_from_zip", { build: activeBuild, githubToken, zipPath: selected });
        if (entries.length > 0) {
          setManifest(prev => {
            if (!prev) return prev;
            const newPaths = new Set(entries.map(m => m.path));
            const kept = prev.mods.filter(m => !newPaths.has(m.path));
            return { ...prev, mods: [...kept, ...entries] };
          });
        }
        notify(`ZIP распакован: ${entries.length} файлов загружено. Нажмите «Commit», чтобы сохранить manifest.`);
        await loadRepoTree();
      } catch (e) { notify(`Ошибка загрузки ZIP: ${String(e)}`); }
      finally {
        if (uploadPollRef.current !== null) { window.clearInterval(uploadPollRef.current); uploadPollRef.current = null; }
        setUploadingBuild(false);
      }
    } catch (e) { notify(String(e)); }
  };

  const commitManifest = async () => {
    if (!manifest) return;
    setSaving(true);
    setMessage("Отправляю manifest сборки на GitHub...");
    try {
      const result = await invoke<string>("commit_build_manifest", {
        build: activeBuild,
        githubToken,
        manifest,
      });
      setMessage(result);
      const newDiscordUrl = manifest.discord_url || "";
      invoke("update_cached_discord_url", { modpackName: activeBuild, discordUrl: newDiscordUrl }).catch(() => {});
      if (onDiscordUrlChange) onDiscordUrlChange(newDiscordUrl);
    } catch (e) {
      setMessage(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="settings-panel admin-panel" ref={panelRef}>

      <h2 style={{ marginBottom: 10, fontWeight: 800, fontSize: 22 }}>Админ-панель</h2>

      <div className="admin-main-tabs">

        <button className={`admin-main-tab ${activeTab === "accounts" ? "active" : ""}`} onClick={() => setActiveTab("accounts")}>
          <span>Пароли</span>
          <small>Оффлайн-аккаунты игроков</small>
        </button>
        {isOwner && (
          <button className={`admin-main-tab ${activeTab === "builds" ? "active" : ""}`} onClick={() => setActiveTab("builds")}>
            <span>Сборки</span>
            <small>DanganVerse и MiniGames: моды, версия, loader</small>
          </button>
        )}

      </div>

      <div className="admin-token-box">
        <div className="admin-account-name">GitHub token</div>
        <input
          className="admin-password-input"
          type="password"
          value={githubToken}
          onChange={e => saveToken(e.target.value)}
          placeholder="github_pat_... с Contents: Read and write"
        />
      </div>

      {activeTab === "accounts" && (
        <>
          <div className="admin-note">
            Здесь можно менять пароли и удалять игроков. После подтверждения лаунчер сам зашифрует
            <b> public/auth/offline_accounts.danganverseenc</b> и отправит commit в GitHub.
          </div>

          <div className="admin-account-list">
            {accounts.map((account, index) => {
              const visible = !!showPasswords[account.username];
              return (
                <div className="admin-account-row" key={account.username}>
                  <div className="admin-account-name">
                    {account.username}
                    {account.username.toLowerCase() === username.toLowerCase() && <span className="admin-mod-count" style={{ fontSize: 9, marginLeft: 6 }}>ВЫ</span>}
                  </div>
                  <input className="admin-password-input" type={visible ? "text" : "password"} value={account.password} onChange={e => updatePassword(index, e.target.value)} />
                  {isOwner && account.username.toLowerCase() !== ADMIN_NAME.toLowerCase() && (
                    <label className="admin-mod-enabled admin-role-toggle" title="Модератор может управлять пользователями, но не сборками">
                      <input
                        type="checkbox"
                        checked={(account.role || "").toLowerCase() === "moderator"}
                        onChange={e => setAccounts(prev => prev.map((row, i) => i === index ? { ...row, role: e.target.checked ? "moderator" : "" } : row))}
                      />
                      <span>Модер</span>
                    </label>
                  )}
                  <button className="settings-btn compact" onClick={() => setShowPasswords(prev => ({ ...prev, [account.username]: !visible }))}>{visible ? "Скрыть" : "Показать"}</button>
                  <button className="settings-btn danger compact" disabled={account.username.toLowerCase() === ADMIN_NAME.toLowerCase()} onClick={() => deleteAccount(account)}>Удалить</button>

                </div>
              );
            })}
          </div>
          <button className="settings-btn accent" onClick={commitChanges} disabled={saving || !githubToken.trim()}>{saving ? "Отправка..." : "Подтвердить и отправить commit"}</button>
        </>
      )}

      {activeTab === "builds" && isOwner && (
        <div className="admin-build-panel">
          <div className="admin-build-tabs">
            {BUILD_NAMES.map(build => (
              <button key={build} className={`admin-build-tab ${activeBuild === build ? "active" : ""}`} onClick={() => setActiveBuild(build)}>
                {build === "danganverse" ? "DanganVerse" : build}
              </button>
            ))}
          </div>

          {!githubToken.trim() && <div className="admin-message">Введите GitHub token выше, чтобы загрузить настройки сборок.</div>}
          {githubToken.trim() && !manifest && <div className="admin-message">Загружаю manifest сборки...</div>}

          {manifest && (
            <>
              <div className="admin-download-dir-row">
                <div>
                  <div className="admin-account-name">Папка сохранения</div>
                  <div className="admin-download-dir-path">{downloadDir || "Не выбрана"}</div>
                </div>
                <button className="settings-btn compact" onClick={chooseDownloadDir}>Изменить</button>
              </div>

              <div className="admin-build-settings" style={{ marginBottom: 16 }}>
                <label>
                  Версия Minecraft
                  <select className="admin-password-input" value={manifest.minecraft_version} onChange={e => updateManifest({ minecraft_version: e.target.value })}>
                    {!availableVersions.includes(manifest.minecraft_version) && <option value={manifest.minecraft_version}>{manifest.minecraft_version}</option>}
                    {availableVersions.length > 0 ? availableVersions.map(v => <option key={v} value={v}>{v}</option>) : <option value={manifest.minecraft_version}>{manifest.minecraft_version}</option>}
                  </select>
                </label>
                <label>Загрузчик<select className="admin-password-input" value={manifest.loader} onChange={e => updateManifest({ loader: e.target.value })}>{LOADERS.map(l => <option key={l} value={l}>{l}</option>)}</select></label>
                <label>Версия загрузчика<input className="admin-password-input" value={manifest.loader_version || ""} onChange={e => updateManifest({ loader_version: e.target.value })} placeholder="latest" /></label>
                <label>IP сервера<input className="admin-password-input" value={manifest.server_ip || ""} onChange={e => updateManifest({ server_ip: e.target.value || undefined })} placeholder="play.example.com:25565" /></label>
                <label>Discord<input className="admin-password-input" value={manifest.discord_url || ""} onChange={e => updateManifest({ discord_url: e.target.value || undefined })} placeholder="https://discord.gg/..." /></label>
              </div>

              <div className="admin-drop-zone" onDragOver={e => e.preventDefault()} onDrop={onDropMod}>
                <div style={{ fontSize: 12, opacity: 0.7, marginBottom: 4 }}>
                  {uploadingMod ? "Загрузка файла на GitHub..." : uploadingBuild ? "Загрузка на GitHub..." : "Перетащите файлы сюда или выберите ниже"}
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <button className="settings-btn compact" type="button" onClick={chooseModFiles} disabled={uploadingMod || uploadingBuild} title=".jar мод или .zip (ресурспак/шейдер)">
                    .jar / .zip файл
                  </button>
                  <button className="settings-btn compact" type="button" onClick={chooseAndUploadZip} disabled={uploadingMod || uploadingBuild} title="ZIP распаковывается: его файлы (mods/, config/, resourcepacks/, shaderpacks/, emotes/, options.txt и т.д.) коммитятся на GitHub по своим путям. Папки сохраняются.">
                    {uploadingBuild ? "Загрузка..." : "📦 ZIP-сборка"}
                  </button>
                  <button className="settings-btn compact" type="button" onClick={uploadModpackFolder} disabled={uploadingMod || uploadingBuild} title="Папка со структурой сборки">
                    {uploadingBuild ? "Загрузка..." : "📁 Папка сборки"}
                  </button>
                </div>
              </div>

              {uploadingBuild && uploadProgress && (
                <div className="admin-upload-progress">
                  <div className="admin-upload-bar-wrap">
                    <div className="admin-upload-bar" style={{ width: uploadProgress.total > 0 ? `${Math.round(uploadProgress.done / uploadProgress.total * 100)}%` : "0%" }} />
                  </div>
                  <div className="admin-upload-status">
                    {uploadProgress.total > 0 ? `${uploadProgress.done} / ${uploadProgress.total}` : "…"} — <span style={{ opacity: 0.75 }}>{uploadProgress.current}</span>
                  </div>
                  {uploadProgress.errors.length > 0 && <div className="admin-upload-errors">Ошибки: {uploadProgress.errors.slice(-3).join(" | ")}</div>}
                </div>
              )}

              <div style={{ marginTop: 20 }}>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 6 }}>
                  <span style={{ fontSize: 13, fontWeight: 700 }}>Файлы репозитория</span>
                  <button className="settings-btn compact" onClick={loadRepoTree} disabled={treeLoading}>
                    {treeLoading ? "Загрузка..." : "🔄 Обновить"}
                  </button>
                </div>
                {treeError && <div className="admin-message" style={{ color: "#e74c3c", fontSize: 12 }}>{treeError}</div>}
                {treeLoading && <div className="admin-message" style={{ fontSize: 12, opacity: 0.7 }}>Загружаю дерево файлов с GitHub…</div>}
                {!treeLoading && builtTree.length === 0 && !treeError && (
                  <div style={{ fontSize: 12, opacity: 0.45, padding: "8px 0" }}>
                    Нажмите «Обновить», чтобы загрузить структуру сборки с GitHub
                  </div>
                )}
                {!treeLoading && builtTree.length > 0 && (
                  <div style={{ background: "rgba(0,0,0,0.22)", borderRadius: 8, padding: "4px 0", maxHeight: 500, overflowY: "auto", border: "1px solid rgba(255,255,255,0.06)" }}>
                    {builtTree.map(node => renderTreeNode(node, 0))}
                  </div>
                )}
              </div>

              <div style={{ marginTop: 16 }}>
                <button
                  className="settings-btn compact"
                  style={{ width: "100%", textAlign: "left", fontSize: 12, justifyContent: "flex-start" }}
                  onClick={() => setShowModList(v => !v)}
                >
                  {showModList ? "▼" : "▶"} Моды в манифесте ({manifest.mods.length}) — вкл/выкл, удаление из списка
                </button>
                {showModList && (
                  <>
                    <div className="admin-mod-search" style={{ marginTop: 8 }}>
                      <input value={modSearch} onChange={e => setModSearch(e.target.value)} placeholder={`Поиск... (всего: ${manifest.mods.length})`} />
                    </div>
                    <div className="admin-mod-list" onDragOver={e => e.preventDefault()} onDrop={onDropMod}>
                      {manifest.mods
                        .map((mod, originalIndex) => ({ mod, originalIndex }))
                        .filter(({ mod }) => mod.name.toLowerCase().includes(modSearch.toLowerCase().trim()))
                        .map(({ mod, originalIndex }) => (
                        <div className="admin-mod-row" key={`${mod.name}-${mod.sha1}`}>
                          <input className="admin-password-input" value={mod.name} onChange={e => updateMod(originalIndex, { name: e.target.value, path: `mods/${e.target.value}`, url: mod.url.replace(/mods\/[^/]+$/, `mods/${encodeURIComponent(e.target.value)}`) })} />
                          <div className="admin-mod-meta">{formatSize(mod.size)}</div>
                          <label className="admin-mod-enabled"><input type="checkbox" checked={mod.enabled} onChange={e => updateMod(originalIndex, { enabled: e.target.checked })} /><span>Вкл.</span></label>
                          <button className="settings-btn compact" onClick={() => downloadMod(mod)}>Скачать</button>
                          <button className="settings-btn danger compact" onClick={() => deleteMod(mod)}>Удалить</button>
                        </div>
                      ))}
                    </div>
                  </>
                )}
              </div>

              <div className="admin-build-floating-actions">
                <button className="settings-btn" onClick={downloadBuild}>Скачать сборку</button>
                <button className="settings-btn accent" onClick={commitManifest} disabled={saving || !githubToken.trim()}>{saving ? "Отправка..." : "Подтвердить и отправить commit"}</button>
              </div>
            </>
          )}
        </div>
      )}

      {message && <div className="admin-message">{message}</div>}
      {toasts.length > 0 && (
        <div className="notification-stack admin-toast-stack">
          {toasts.map(item => <div key={item.id} className="notification admin-toast">{item.text}</div>)}
        </div>
      )}

    </div>

  );
}