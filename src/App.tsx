import { useEffect, useLayoutEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import Titlebar from "./components/Titlebar";
import Sidebar, { type Page } from "./components/Sidebar";
import ParticlesBg from "./components/ParticlesBg";
import AuthPanel from "./components/AuthPanel";
import GamePanel from "./components/GamePanel";
import SettingsPanel from "./components/SettingsPanel";
import CustomModpackPanel from "./components/CustomModpackPanel";
import UpdateOverlay from "./components/UpdateOverlay";
import AdminPanel from "./components/AdminPanel";
import ModpackSettingsPanel from "./components/ModpackSettingsPanel";

interface Account {
  username: string;
  uuid: string;
  access_token: string;
  account_type: string;
  is_admin?: boolean;
  is_owner?: boolean;
  role?: string;
}

interface JavaInfo {
  path: string;
  version: string;
  found: boolean;
}

interface LaunchProgress {
  stage: string;
  progress: number;
  total: number;
  message: string;
}

interface UpdateInfo {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  download_url: string;
  installer_url: string;
  release_notes: string;
  file_size: number;
}

type Theme = "light" | "dark";

export interface CustomModpack {
  name: string;
  loader: string;
  mc_version: string;
  loader_version: string;
  max_memory: number;
  jvm_args: string;
  created_at: string;
  game_dir: string;
}


let appInitialized = false;

const DISCORD_URL_STORAGE = "danganverse_discord_url";
const DISCORD_URL_FALLBACK = "https://discord.gg/yvFE4rjnjf";

export default function App() {
  const [account, setAccount] = useState<Account | null>(null);
  const [currentPage, setCurrentPage] = useState<Page>("danganverse");
  const [configurePage, setConfigurePage] = useState<Page | null>(null);
  const [loading, setLoading] = useState(true);
  const [javaPath, setJavaPath] = useState("");
  const [javaVersion, setJavaVersion] = useState("");
  const [maxMemory, setMaxMemory] = useState(4096);
  const [jvmArgs, setJvmArgs] = useState("");
  const [gpuMode, setGpuMode] = useState("auto");
  const [theme, setTheme] = useState<Theme>(() => {
    const saved = localStorage.getItem("darkspark_theme");
    return saved === "dark" || saved === "light" ? saved : "dark";
  });
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null);
  const [notification, setNotification] = useState("");
  const [pendingUpdate, setPendingUpdate] = useState<UpdateInfo | null>(null);
  const [customModpacks, setCustomModpacks] = useState<CustomModpack[]>([]);
  const [allowMultipleInstances, setAllowMultipleInstances] = useState(false);
  const [closeLauncherOnGameStart, setCloseLauncherOnGameStart] = useState(true);
  const [reopenLauncherAfterGameClose, setReopenLauncherAfterGameClose] = useState(true);
  const [globalLaunchProgress, setGlobalLaunchProgress] = useState<LaunchProgress | null>(null);


  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("darkspark_theme", theme);
    invoke("save_theme", { theme }).catch(() => {});
  }, [theme]);

  useEffect(() => {
    invoke<string>("get_saved_theme").then(saved => {
      if (saved === "dark" || saved === "light") setTheme(saved as Theme);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    const preventDefaultContextMenu = (event: MouseEvent) => event.preventDefault();
    window.addEventListener("contextmenu", preventDefaultContextMenu, { capture: true });
    return () => window.removeEventListener("contextmenu", preventDefaultContextMenu, true);
  }, []);

  useEffect(() => {
    if (appInitialized) return;
    appInitialized = true;
    initializeApp();
  }, []);

  useEffect(() => {
    const timer = window.setInterval(async () => {
      try {
        const progress = await invoke<LaunchProgress | null>("get_launch_progress");
        if (progress && progress.stage !== "done") setGlobalLaunchProgress(progress);
        else if (progress?.stage === "done") setGlobalLaunchProgress(null);
      } catch (e) {
        // error ignored
      }
    }, 700);
    return () => window.clearInterval(timer);
  }, []);

  const initializeApp = async () => {
    setLoading(true);
    try {
      const savedMemory = localStorage.getItem("darkspark_memory");
      const savedJavaPath = localStorage.getItem("darkspark_java_path");
      const savedJavaVersion = localStorage.getItem("darkspark_java_version");
      const savedJvmArgs = localStorage.getItem("darkspark_jvm_args");
      const savedGpuMode = localStorage.getItem("darkspark_gpu_mode");
      const savedTheme = localStorage.getItem("darkspark_theme") as Theme | null;
      const savedAllowMultipleInstances = localStorage.getItem("darkspark_allow_multiple_instances");
      const savedCloseLauncher = localStorage.getItem("darkspark_close_launcher_on_game_start");
      const savedReopenLauncher = localStorage.getItem("darkspark_reopen_launcher_after_game_close");

      if (savedMemory) { const m = parseInt(savedMemory); if (!isNaN(m)) setMaxMemory(Math.max(1024, Math.min(16384, m))); }
      if (savedJavaPath) setJavaPath(savedJavaPath);
      if (savedJavaVersion) setJavaVersion(savedJavaVersion);
      if (savedJvmArgs) setJvmArgs(savedJvmArgs);
      if (savedGpuMode) setGpuMode(savedGpuMode);
      if (savedTheme === "dark" || savedTheme === "light") setTheme(savedTheme);
      if (savedAllowMultipleInstances) setAllowMultipleInstances(savedAllowMultipleInstances === "true");
      if (savedCloseLauncher) setCloseLauncherOnGameStart(savedCloseLauncher === "true");
      if (savedReopenLauncher) setReopenLauncherAfterGameClose(savedReopenLauncher === "true");

      await loadCustomModpacks();

      const loggingEnabled = localStorage.getItem("darkspark_logging") !== "false";
      try { await invoke("set_logging_enabled", { enabled: loggingEnabled }); } catch (e) {
        // error ignored
      }

      try { const dataUrl = await invoke<string | null>("get_avatar"); if (dataUrl) setAvatarUrl(dataUrl); } catch (e) {
        // error ignored
      }

      const savedAccount = await invoke<Account | null>("get_saved_account");
      if (savedAccount) setAccount(savedAccount);

      if (!savedJavaPath) {
        try { const j = await invoke<JavaInfo>("find_java"); if (j.found) handleJavaChange(j.path, j.version); } catch (e) {
          // error ignored
        }
      }

      const justUpdated = await invoke<boolean>("check_just_updated").catch(() => false);
      if (!justUpdated) {
        try {
          const updateInfo = await invoke<UpdateInfo>("check_launcher_update");
          if (updateInfo.update_available) setPendingUpdate(updateInfo);
        } catch (e) {
          console.error("Update check failed:", e);
        }
      }
    } catch (e) {
      console.error("Init failed:", e);
    } finally {
      setLoading(false);
    }
  };

  const loadCustomModpacks = async () => {
    try {
      const list = await invoke<CustomModpack[]>("get_custom_modpacks");
      setCustomModpacks(list);
    } catch (e) {
        // error ignored
      }
  };

  const deleteCustomModpack = async (name: string) => {
    if (!confirm(`Удалить модпак «${name}»?`)) return;
    try {
      await invoke("delete_custom_modpack", { name });
      await loadCustomModpacks();
      setCurrentPage("danganverse");
      showNotification(`Модпак «${name}» удалён`);
    } catch (e) {
      showNotification(String(e));
    }
  };

  const deleteBuiltinModpack = async (page: Page) => {
    if (page !== "danganverse") return;
    const title = "danganverse";
    if (!confirm(`Удалить установленную сборку «${title}» с компьютера? Лаунчер останется.`)) return;
    try {
      await invoke("delete_builtin_modpack", { modpackName: page });
      showNotification(`Сборка «${title}» удалена`);
    } catch (e) {
      showNotification(String(e));
    }
  };

  const toggleTheme = () => {
    setTheme(t => t === "light" ? "dark" : "light");
  };

  const handleLogin = (acc: Account) => {
    setAccount(acc);
    showNotification(`Добро пожаловать, ${acc.username}`);
  };

  const handleLogout = async () => {
    try { await invoke("logout"); setAccount(null); showNotification("Вы вышли из аккаунта"); } catch (e) {
        // error ignored
      }
  };

  const handleJavaChange = (path: string, version: string) => {
    setJavaPath(path); setJavaVersion(version);
    localStorage.setItem("rpw_java_path", path);
    localStorage.setItem("rpw_java_version", version);
  };

  const handleMemoryChange = (mem: number) => {
    setMaxMemory(mem);
    localStorage.setItem("rpw_memory", String(mem));
  };

  const handleJvmArgsChange = (args: string) => {
    setJvmArgs(args);
    localStorage.setItem("rpw_jvm_args", args);
  };

  const handleGpuModeChange = (mode: string) => {
    setGpuMode(mode);
    localStorage.setItem("rpw_gpu_mode", mode);
  };

  const handleAllowMultipleInstancesChange = (value: boolean) => {
    setAllowMultipleInstances(value);
    localStorage.setItem("rpw_allow_multiple_instances", String(value));
  };

  const handleCloseLauncherOnGameStartChange = (value: boolean) => {
    setCloseLauncherOnGameStart(value);
    localStorage.setItem("rpw_close_launcher_on_game_start", String(value));
  };

  const handleReopenLauncherAfterGameCloseChange = (value: boolean) => {
    setReopenLauncherAfterGameClose(value);
    localStorage.setItem("rpw_reopen_launcher_after_game_close", String(value));
  };

  const showNotification = (msg: string) => {
    setNotification(msg);
    setTimeout(() => setNotification(""), 3500);
  };

  if (loading) {
    return (
      <div className="app-container" style={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
        <ParticlesBg />
        <motion.div
          style={{ zIndex: 2, display: "flex", flexDirection: "column", alignItems: "center", gap: 18 }}
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.5 }}
        >
          <motion.img
            src="/icons/Inside.png"
            alt="DanganVerse"
            style={{ width: 72, height: 72, borderRadius: "18px", objectFit: "cover" }}
            animate={{ boxShadow: ["0 0 20px rgba(212,121,58,0.3)", "0 0 50px rgba(212,121,58,0.6)", "0 0 20px rgba(212,121,58,0.3)"] }}
            transition={{ duration: 2, repeat: Infinity }}
          />
          <div style={{ color: "var(--text-muted)", fontSize: 13 }}>Загрузка...</div>
        </motion.div>
      </div>
    );
  }

  return (
    <div className="app-container">
      <ParticlesBg />

      <AnimatePresence>
        {pendingUpdate && (
          <UpdateOverlay
            updateInfo={pendingUpdate}
            onSkip={() => setPendingUpdate(null)}
            theme={theme}
          />
        )}
      </AnimatePresence>

      <Titlebar theme={theme} onThemeToggle={toggleTheme} />

      <div className="main-layout" style={{ position: "relative", zIndex: 1 }}>
        <Sidebar
          currentPage={currentPage}
          onPageChange={(page) => { setConfigurePage(null); setCurrentPage(page); }}
          account={account}
          onLogout={handleLogout}
          avatarUrl={avatarUrl}
          customModpacks={customModpacks}
          onConfigurePage={(page) => {
            if (page === "custom") setCurrentPage("custom");
            else setConfigurePage(page);
          }}
          onDeleteBuiltinModpack={deleteBuiltinModpack}
          onDeleteCustomModpack={deleteCustomModpack}
          discordUrl={localStorage.getItem(DISCORD_URL_STORAGE) || DISCORD_URL}
        />

        <div className="content-area">
          
          <AnimatePresence>
            {!account && !loading && (
              <AuthPanel onLogin={handleLogin} />
            )}
          </AnimatePresence>

          <AnimatePresence mode="wait">
            {configurePage && account ? (
              <motion.div
                key={`configure-${configurePage}`}
                className="game-panel"
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                transition={{ duration: 0.3 }}
              >
                <ModpackSettingsPanel
                  page={configurePage}
                  customModpacks={customModpacks}
                  onBack={() => setConfigurePage(null)}
                  onChanged={loadCustomModpacks}
                />
              </motion.div>
            ) : currentPage === "settings" && account ? (
              <motion.div
                key="settings"
                className="game-panel"
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                transition={{ duration: 0.3 }}
              >
                <SettingsPanel
                  javaPath={javaPath}
                  javaVersion={javaVersion}
                  maxMemory={maxMemory}
                  jvmArgs={jvmArgs}
                  gpuMode={gpuMode}
                  theme={theme}
                  avatarUrl={avatarUrl}
                  username={account.username}
                  onJavaChange={handleJavaChange}
                  onMemoryChange={handleMemoryChange}
                  onJvmArgsChange={handleJvmArgsChange}
                  onGpuModeChange={handleGpuModeChange}
                  allowMultipleInstances={allowMultipleInstances}
                  closeLauncherOnGameStart={closeLauncherOnGameStart}
                  reopenLauncherAfterGameClose={reopenLauncherAfterGameClose}
                  onAllowMultipleInstancesChange={handleAllowMultipleInstancesChange}
                  onCloseLauncherOnGameStartChange={handleCloseLauncherOnGameStartChange}
                  onReopenLauncherAfterGameCloseChange={handleReopenLauncherAfterGameCloseChange}
                  onThemeChange={setTheme}
                  onAvatarChange={setAvatarUrl}
                />
              </motion.div>
            ) : currentPage === "admin" && account?.is_admin ? (
              <motion.div
                key="admin"
                className="game-panel"
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20 }}
                transition={{ duration: 0.3 }}
              >
                <AdminPanel username={account.username} isOwner={!!account.is_owner} />
              </motion.div>
            ) : currentPage === "custom" ? (
              <CustomModpackPanel key="custom" onInstalled={loadCustomModpacks} />
            ) : currentPage !== "settings" ? (
              <GamePanel
                key={currentPage}
                page={currentPage}
                customModpacks={customModpacks}
                account={account}
                javaPath={javaPath}
                maxMemory={maxMemory}
                jvmArgs={jvmArgs}
                gpuMode={gpuMode}
                allowMultipleInstances={allowMultipleInstances}
                closeLauncherOnGameStart={closeLauncherOnGameStart}
                reopenLauncherAfterGameClose={reopenLauncherAfterGameClose}
              />
            ) : (
              <GamePanel
                key="fallback"
                page="danganverse"
                account={account}
                javaPath={javaPath}
                maxMemory={maxMemory}
                jvmArgs={jvmArgs}
                gpuMode={gpuMode}
                allowMultipleInstances={allowMultipleInstances}
                closeLauncherOnGameStart={closeLauncherOnGameStart}
                reopenLauncherAfterGameClose={reopenLauncherAfterGameClose}
              />
            )}
          </AnimatePresence>
        </div>
      </div>

      <AnimatePresence>
        {globalLaunchProgress && currentPage !== "danganverse" && !currentPage.startsWith("custom:") && (
          <motion.div
            className="global-launch-progress"
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 16 }}
          >
            <strong>Установка / запуск сборки</strong>
            <span>{globalLaunchProgress.message}</span>
            {globalLaunchProgress.total > 0 && (
              <div className="global-progress-bar"><i style={{ width: `${Math.round((globalLaunchProgress.progress / globalLaunchProgress.total) * 100)}%` }} /></div>
            )}
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {notification && (
          <motion.div
            className="notification"
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 16 }}
            transition={{ duration: 0.25 }}
          >
            {notification}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
