import { useState, useEffect, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import type { Page } from "./Sidebar";
import type { CustomModpack } from "../App";

interface Account {
  username: string;
  uuid: string;
  access_token: string;
  account_type: string;
}

interface LaunchProgress {
  stage: string;
  progress: number;
  total: number;
  message: string;
}

interface GamePanelProps {
  page: Page;
  account: Account | null;
  javaPath: string;
  maxMemory: number;
  jvmArgs?: string;
  gpuMode?: string;
  customModpacks?: CustomModpack[];
  allowMultipleInstances?: boolean;
  closeLauncherOnGameStart?: boolean;
  reopenLauncherAfterGameClose?: boolean;
}

interface ModpackConfig {
  title: string;
  description: string;
  githubRepo: string;
  modpackName: string;
  defaultVersion: string;
  mcVersion: string;
  locked?: boolean;
  bg: string[];
}

const MODPACK_CONFIGS: Record<string, ModpackConfig> = {
  rpworld: {
    title: "RPWorld",
    description: "Погрузитесь в мир ролевых приключений на Forge 1.20.1 с уникальными модами, квестами и захватывающим геймплеем.",
    githubRepo: "Sadoul/rpworld",
    modpackName: "rpworld",
    defaultVersion: "forge-1.20.1",
    mcVersion: "1.20.1",
    bg: ["/backgrounds/rpworld.jpg", "/backgrounds/rpworld2.jpg"],
  },
  minigames: {
    title: "Мини-игры",
    description: "BedWars, SkyWars и другие мини-игры. В разработке — скоро!",
    githubRepo: "Sadoul/minigames",
    modpackName: "minigames",
    defaultVersion: "1.20.1",
    mcVersion: "1.20.1",
    locked: true,
    bg: ["/backgrounds/minigames.jpg"],
  },
};

export default function GamePanel({
  page,
  account,
  javaPath,
  maxMemory,
  jvmArgs = "",
  gpuMode = "auto",
  customModpacks = [],
  allowMultipleInstances = false,
  closeLauncherOnGameStart = true,
  reopenLauncherAfterGameClose = true,
}: GamePanelProps) {
  const [launching, setLaunching] = useState(false);
  const [gameRunning, setGameRunning] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [progress, setProgress] = useState<LaunchProgress | null>(null);
  const [status, setStatus] = useState<"ready" | "downloading" | "update">("ready");
  const [downloadProgress, setDownloadProgress] = useState<{ downloaded: number; total: number; message: string } | null>(null);
  const [error, setError] = useState("");
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const progressInterval = useRef<number | null>(null);

  const customPackName = page.startsWith("custom:") ? page.slice("custom:".length) : null;
  const customPack = customPackName ? customModpacks.find(pack => pack.name === customPackName) : null;
  const config = customPack ? {
    title: customPack.name,
    description: `${customPack.loader.toUpperCase()} · Minecraft ${customPack.mc_version}${customPack.loader_version ? ` · ${customPack.loader_version}` : ""}`,
    githubRepo: "",
    modpackName: customPack.name,
    defaultVersion: customPack.loader === "vanilla" ? customPack.mc_version : `${customPack.loader}-${customPack.mc_version}`,
    mcVersion: customPack.mc_version,
    bg: ["/backgrounds/custom.jpg"],
  } : (MODPACK_CONFIGS[page] || MODPACK_CONFIGS.rpworld);


  const bgImage = useMemo(() => {
    if (!config.bg || config.bg.length === 0) return null;
    return config.bg[Math.floor(Math.random() * config.bg.length)];
  }, [page]);


  useEffect(() => {
    let cancelled = false;
    const wasDownloading = { current: status === "downloading" };

    const poll = async () => {

      try {
        const lp = await invoke<LaunchProgress | null>("get_launch_progress");
        if (!cancelled) {
          if (lp && lp.stage !== "done") {
            setLaunching(true);
            setProgress(lp);
          } else if (lp && lp.stage === "done") {
            setProgress(lp);
            window.setTimeout(() => {
              if (!cancelled) { setLaunching(false); setProgress(null); }
            }, 1500);
          }
        }
      } catch 


      try {
        const dp = await invoke<{ downloaded: number; total: number; message: string } | null>("get_download_progress");
        if (!cancelled) {
          if (dp) {
            const finished = dp.message.includes("установлена");
            const active = !finished && (dp.total === 0 || dp.downloaded < dp.total);
            if (active) {
              setStatus("downloading");
              setDownloadProgress(dp);
              wasDownloading.current = true;
            } else if (finished && wasDownloading.current) {
              wasDownloading.current = false;
              setDownloadProgress(null);
              setStatus("ready");
              if (!config.locked) checkModpackUpdate();
            }
          }
        }
      } catch 


      try {
        if (!cancelled) setGameRunning(await invoke<boolean>("is_game_running"));
      } catch 
    };

    if (!config.locked) checkModpackUpdate();
    poll();
    const timer = window.setInterval(poll, 700);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
      if (progressInterval.current) clearInterval(progressInterval.current);
    };
  }, [page]);

  const checkModpackUpdate = async () => {
    setCheckingUpdate(true);
    try {
      const update = await invoke<any>("check_modpack_update", {
        modpackName: config.modpackName,
        githubRepo: config.githubRepo,
      });
      setStatus(update ? "update" : "ready");
    } catch {
      setStatus("ready");
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handlePlay = async () => {
    if (status === "update") {
      setStatus("downloading");
      setError("");
      try {
        const update = await invoke<any>("check_modpack_update", {
          modpackName: config.modpackName,
          githubRepo: config.githubRepo,
        });
        if (update) {
          await invoke("download_modpack", {
            modpackName: update.name,
            downloadUrl: update.download_url,
            version: update.version,
            minecraftVersion: update.minecraft_version,
          });
        }
        setStatus("ready");
      } catch (err) {
        setError(String(err));
        setStatus("ready");
      }
      return;
    }

    if (!account) {
      setError("Войдите в аккаунт перед запуском");
      return;
    }

    if (!allowMultipleInstances && gameRunning) {
      setError("Minecraft уже запущен. Включите «Разрешить твинки» в настройках, если хотите открыть ещё один клиент.");
      return;
    }

    setLaunching(true);
    setError("");
    setProgress(null);

    progressInterval.current = window.setInterval(async () => {
      try {
        const p = await invoke<LaunchProgress | null>("get_launch_progress");
        if (p) {
          setProgress(p);
          if (p.stage === "done") {
            if (progressInterval.current) clearInterval(progressInterval.current);
            setTimeout(() => { setLaunching(false); setProgress(null); }, 2000);
          }
        }
      } catch 
    }, 500);

    try {
      const gameDir = customPack?.game_dir || await invoke<string>("get_builtin_modpack_dir", { modpackName: config.modpackName });

      const savedBuiltinMemory = !customPack ? Number(localStorage.getItem(`rpw_modpack_memory_${config.modpackName}`)) : NaN;
      const effectiveMemory = customPack?.max_memory ?? (!Number.isNaN(savedBuiltinMemory) ? savedBuiltinMemory : maxMemory);

      await invoke("launch_game", {
        username: account.username,
        uuid: account.uuid,
        accessToken: account.access_token,
        version: config.defaultVersion,
        javaPath,
        maxMemory: effectiveMemory,
        gameDir,
        jvmArgs: customPack?.jvm_args ?? jvmArgs,
        gpuMode,
        allowMultipleInstances,
        closeLauncherOnGameStart,
        reopenLauncherAfterGameClose,
      });
    } catch (err) {
      setError(String(err));
      setLaunching(false);
      if (progressInterval.current) clearInterval(progressInterval.current);
    }
  };

  const handleCancel = async () => {
    setCancelling(true);
    try {
      await invoke("cancel_launch");
      await invoke("cancel_download");
    } catch 
    setLaunching(false);
    setStatus("ready");
    setProgress(null);
    setDownloadProgress(null);
    if (progressInterval.current) clearInterval(progressInterval.current);
    setCancelling(false);
  };

  const copyError = async () => {
    if (!error) return;
    try {
      await navigator.clipboard.writeText(error);
      setError(`${error}\n\n[Скопировано в буфер обмена]`);
    } catch {
      setError(`${error}\n\n[Не удалось скопировать автоматически — выделите текст вручную]`);
    }
  };

  const progressPercent = progress && progress.total > 0
    ? Math.round((progress.progress / progress.total) * 100)
    : 0;

  return (
    <motion.div
      className={`game-panel ${bgImage ? "has-bg" : ""}`}
      key={page}
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -15 }}
      transition={{ duration: 0.35, ease: "easeOut" }}
    >
      
      {bgImage && (
        <div
          className="panel-bg"
          style={{ backgroundImage: `url(${bgImage})` }}
        />
      )}

      
      {config.locked && (
        <div className="locked-overlay">
          <div className="locked-overlay-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" width="40" height="40">
              <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.5" opacity="0.4"/>
              <path d="M12 7v5" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
              <circle cx="12" cy="15.5" r="1" fill="currentColor"/>
            </svg>
          </div>
          <div className="locked-overlay-title">В разработке</div>
          <div className="locked-overlay-sub">Скоро будет доступно</div>
        </div>
      )}

      <div className="game-panel-header">
        <motion.h2
          initial={{ opacity: 0, x: -10 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: 0.1, duration: 0.3 }}
        >
          {config.title}
        </motion.h2>
        <motion.div
          className="description-glass-card"
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.18, duration: 0.3 }}
        >
          <p className="description">{config.description}</p>
        </motion.div>
      </div>

      <div style={{ flex: 1 }} />

      {(launching || status === "downloading" || error || (!javaPath && !config.locked)) && (
        <div className="game-play-area">
          <AnimatePresence mode="wait">
            {launching || status === "downloading" ? (
              <motion.div
                key="launching"
                className="progress-container"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.95 }}
                transition={{ duration: 0.25 }}
                style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 16, width: "100%" }}
              >
                {(progress || downloadProgress) && (
                  <>
                    <div className="progress-bar-wrapper" style={{ width: "100%" }}>
                      <motion.div
                        className="progress-bar-fill"
                        initial={{ width: "0%" }}
                        animate={{ width: `${progress ? progressPercent : (downloadProgress && downloadProgress.total > 0 ? Math.round((downloadProgress.downloaded / downloadProgress.total) * 100) : 0)}%` }}
                        transition={{ duration: 0.3 }}
                      />
                    </div>
                    <div className="progress-text">
                      {progress ? progress.message : downloadProgress?.message}
                    </div>
                  </>
                )}
                <motion.button
                  className="cancel-button"
                  onClick={handleCancel}
                  disabled={cancelling}
                  whileHover={{ scale: 1.03 }}
                  whileTap={{ scale: 0.97 }}
                >
                  {cancelling ? "Отмена..." : "Отменить"}
                </motion.button>
              </motion.div>
            ) : (
              <motion.div
                key="play"
                initial={{ opacity: 0, scale: 0.92 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.92 }}
                transition={{ duration: 0.3 }}
                style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 14, width: "100%" }}
              >
                {error && (
                  <motion.div
                    className="launch-error-card"
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: "auto" }}
                  >
                    <pre className="launch-error-text">{error}</pre>
                    <motion.button
                      className="copy-error-button"
                      onClick={copyError}
                      whileHover={{ scale: 1.03 }}
                      whileTap={{ scale: 0.97 }}
                    >
                      Скопировать ошибку
                    </motion.button>
                  </motion.div>
                )}

                {!javaPath && !config.locked && (
                  <div className="java-warning">
                    Java не найдена. Настройте Java в настройках.
                  </div>
                )}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      )}

      <div className="game-panel-footer">
        <div className="footer-glass-card">
          <span className="footer-modpack-name">{config.title}</span>
          <span className="footer-mc-version">Minecraft {config.mcVersion} · Forge</span>
        </div>
        <div className="footer-actions">
          <span className={`status-badge ${status === "ready" ? "ready" : status === "update" ? "update" : "downloading"}`}>
            <span className="status-dot" />
            {status === "ready" ? "Готово" : status === "update" ? "Обновление" : "Загрузка"}
          </span>
          {!config.locked && (
            <motion.button
              className="play-button-hero footer-play-button"
              onClick={handlePlay}
              disabled={launching || status === "downloading" || !javaPath || checkingUpdate || config.locked || (!allowMultipleInstances && gameRunning)}
              whileHover={{ scale: 1.03, y: -2 }}
              whileTap={{ scale: 0.97 }}
            >
              <span className="play-button-icon">▶</span>
              <span>
                {checkingUpdate ? "Проверка..."
                  : status === "update" ? "Обновить"
                  : status === "downloading" ? "Скачивание..."
                  : (!allowMultipleInstances && gameRunning) ? "Minecraft уже открыт"
                  : "Играть"}
              </span>
            </motion.button>
          )}
        </div>
      </div>
    </motion.div>
  );
}