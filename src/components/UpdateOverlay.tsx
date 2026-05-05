import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";

interface UpdateInfo {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  download_url: string;
  installer_url: string;
  release_notes: string;
  file_size: number;
}

interface UpdateProgress {
  stage: string;
  downloaded: number;
  total: number;
  speed_kb: number;
  message: string;
}

interface UpdateOverlayProps {
  updateInfo: UpdateInfo;
  onSkip: () => void;
  theme: "light" | "dark";
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} Б`;
  if (bytes < 1_048_576) return `${(bytes / 1024).toFixed(1)} КБ`;
  return `${(bytes / 1_048_576).toFixed(1)} МБ`;
}

function formatSpeed(kbs: number): string {
  if (kbs < 1024) return `${kbs} КБ/с`;
  return `${(kbs / 1024).toFixed(1)} МБ/с`;
}

export default function UpdateOverlay({ updateInfo, theme }: UpdateOverlayProps) {
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [started, setStarted] = useState(false);
  const [error, setError] = useState("");
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    listen<UpdateProgress>("update-progress", (event) => {
      setProgress(event.payload);
    }).then((unlisten) => {
      unlistenRef.current = unlisten;
    });
    return () => { if (unlistenRef.current) unlistenRef.current(); };
  }, []);

  useEffect(() => {
    handleUpdate();
  }, []);

  const handleUpdate = async () => {
    setStarted(true);
    setError("");
    try {
      await invoke("update_launcher");
    } catch (err) {
      setError(String(err));
      setStarted(false);
    }
  };

  const percent =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : 0;

  const isDone = progress?.stage === "done";
  const isApplying = progress?.stage === "applying";
  const isDownloading = progress?.stage === "downloading";

  return (
    <motion.div
      className="update-overlay"
      data-theme={theme}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.35 }}
    >
      
      <div className="update-orbs">
        {[0, 1, 2].map((i) => (
          <motion.div
            key={i}
            className={`update-orb update-orb-${i}`}
            animate={{ scale: [1, 1.18, 1], opacity: [0.6, 1, 0.6] }}
            transition={{ duration: 5 + i * 1.8, repeat: Infinity, delay: i * 1.1 }}
          />
        ))}
      </div>

      <motion.div
        className="update-content"
        initial={{ opacity: 0, y: 24, scale: 0.97 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ delay: 0.12, duration: 0.4, ease: "easeOut" }}
      >
        
        <div style={{ position: "relative", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <motion.div
            className="update-ring update-ring-outer"
            animate={{ scale: [1, 1.22, 1], opacity: [0.5, 0, 0.5] }}
            transition={{ duration: 2.4, repeat: Infinity }}
          />
          <motion.div
            className="update-ring update-ring-inner"
            animate={{ scale: [1, 1.14, 1], opacity: [0.7, 0, 0.7] }}
            transition={{ duration: 2.4, repeat: Infinity, delay: 0.4 }}
          />
          <img
            src="/icons/Inside.png"
            alt="RPWorld"
            className="update-launcher-icon"
          />
        </div>

        
        <div className="update-title-block">
          <motion.div
            className="update-title"
            animate={isApplying ? { opacity: [1, 0.5, 1] } : {}}
            transition={{ duration: 1.5, repeat: Infinity }}
          >
            {isDone
              ? "✓ Обновление применено!"
              : isApplying
              ? "Применение обновления..."
              : "Доступно обновление"}
          </motion.div>

          {!isDone && !isApplying && (
            <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 8, marginTop: 8 }}>
              <span style={{
                padding: "3px 10px", fontSize: 11, fontWeight: 600,
                background: "var(--bg-overlay)", border: "1px solid var(--border)",
                borderRadius: 20, color: "var(--text-muted)"
              }}>
                v{updateInfo.current_version}
              </span>
              <span style={{ color: "var(--accent)", fontSize: 14 }}>→</span>
              <span style={{
                padding: "3px 10px", fontSize: 11, fontWeight: 600,
                background: "var(--accent-glow)", border: "1px solid var(--border-hover)",
                borderRadius: 20, color: "var(--accent)"
              }}>
                v{updateInfo.latest_version}
              </span>
            </div>
          )}

          <div className="update-subtitle" style={{ marginTop: 8 }}>
            {isDone
              ? "Лаунчер перезапустится автоматически"
              : isApplying
              ? "Пожалуйста, не закрывайте лаунчер"
              : updateInfo.file_size
              ? `Размер: ${formatSize(updateInfo.file_size)}`
              : "Загрузка обновления..."}
          </div>
        </div>

        
        <div style={{ width: "100%", height: 1, background: "var(--border)" }} />

        
        <div className="update-progress-block">
          <div className="update-progress-track">
            <motion.div
              className="update-progress-fill"
              style={isApplying ? { width: "100%" } : {}}
              animate={
                isApplying
                  ? { backgroundPosition: ["0% 0%", "200% 0%"] }
                  : { width: `${started ? percent : 0}%` }
              }
              transition={
                isApplying
                  ? { duration: 1.2, repeat: Infinity, ease: "linear" }
                  : { duration: 0.35, ease: "easeOut" }
              }
            />
          </div>

          <div className="update-progress-stats">
            <span>{started ? (progress?.message || "Соединение...") : "Подготовка..."}</span>
            {isDownloading && progress && (
              <span style={{ display: "flex", gap: 10 }}>
                <span className="update-percent">{percent}%</span>
                {progress.speed_kb > 0 && <span>{formatSpeed(progress.speed_kb)}</span>}
              </span>
            )}
          </div>
        </div>

        
        <AnimatePresence>
          {error && (
            <motion.div
              className="update-error"
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
            >
              <div style={{ marginBottom: 10 }}>{error}</div>
              <motion.button
                className="update-retry-btn"
                onClick={handleUpdate}
                whileHover={{ scale: 1.04 }}
                whileTap={{ scale: 0.96 }}
              >
                Повторить
              </motion.button>
            </motion.div>
          )}
        </AnimatePresence>

        
        {updateInfo.release_notes && (
          <motion.div
            className="update-notes"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.4 }}
          >
            {updateInfo.release_notes.slice(0, 400)}
            {updateInfo.release_notes.length > 400 && "..."}
          </motion.div>
        )}
      </motion.div>
    </motion.div>
  );
}