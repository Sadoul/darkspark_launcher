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

  const deleteMod = (mod: BuildFileEntry) => {
    const ok = window.confirm(
      `Удалить мод из сборки?\n\n${mod.name}\n\nФайл останется в GitHub, но после сохранения manifest клиент перестанет скачивать этот мод.`
    );
    if (!ok) return;
    setManifest(prev => prev ? { ...prev, mods: prev.mods.filter(m => m.name !== mod.name) } : prev);
    notify(`Мод ${mod.name} удалён из списка. Нажмите commit, чтобы сохранить изменение.`);
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
      if (newDiscordUrl && onDiscordUrlChange) onDiscordUrlChange(newDiscordUrl);
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
                {build === "danganverse" ? "DanganVerse" : "MiniGames"}
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

              <div className="admin-build-settings" style={{ marginBottom: 28 }}>
                <label>
                  Версия Minecraft
                  <select className="admin-password-input" value={manifest.minecraft_version} onChange={e => updateManifest({ minecraft_version: e.target.value })}>
                    {!availableVersions.includes(manifest.minecraft_version) && (
                      <option value={manifest.minecraft_version}>{manifest.minecraft_version}</option>
                    )}
                    {availableVersions.length > 0 ? (
                      availableVersions.map(v => <option key={v} value={v}>{v}</option>)
                    ) : (
                      <option value={manifest.minecraft_version}>{manifest.minecraft_version}</option>
                    )}
                  </select>
                </label>

                <label>Загрузчик<select className="admin-password-input" value={manifest.loader} onChange={e => updateManifest({ loader: e.target.value })}>{LOADERS.map(loader => <option key={loader} value={loader}>{loader}</option>)}</select></label>
                <label>Версия загрузчика<input className="admin-password-input" value={manifest.loader_version || ""} onChange={e => updateManifest({ loader_version: e.target.value })} placeholder="можно пусто = latest" /></label>
                <label>IP сервера<input className="admin-password-input" value={manifest.server_ip || ""} onChange={e => updateManifest({ server_ip: e.target.value || undefined })} placeholder="play.example.com:25565" /></label>
                <label>Discord<input className="admin-password-input" value={manifest.discord_url || ""} onChange={e => updateManifest({ discord_url: e.target.value || undefined })} placeholder="https://discord.gg/..." /></label>
              </div>

              <div className="admin-drop-zone" onDragOver={e => e.preventDefault()} onDrop={onDropMod}>
                <div>{uploadingMod ? "Загрузка файла на GitHub..." : "Перетащите .jar (мод) или .zip (ресурспак/шейдер) сюда или на список ниже"}</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 6 }}>
                  <button className="settings-btn compact" type="button" onClick={chooseModFiles} disabled={uploadingMod || uploadingBuild}>
                    Выбрать .jar / .zip
                  </button>
                  <button className="settings-btn compact" type="button" onClick={uploadModpackFolder} disabled={uploadingMod || uploadingBuild} title="Загрузить mods/, config/, resourcepacks/, shaderpacks/, schematics/, options.txt на GitHub">
                    {uploadingBuild ? "Загрузка сборки..." : "📁 Загрузить папку сборки"}
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
                  {uploadProgress.errors.length > 0 && (
                    <div className="admin-upload-errors">Ошибки: {uploadProgress.errors.slice(-3).join(" | ")}</div>
                  )}
                </div>
              )}


              <div className="admin-mod-search">
                <input value={modSearch} onChange={e => setModSearch(e.target.value)} placeholder={`Поиск по модам... (всего: ${manifest.mods.length})`} />
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