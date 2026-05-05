import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";

interface McVersion {
  id: string;
  type: "release" | "snapshot" | "old_beta" | "old_alpha";
}

interface LoaderVersion {
  id: string;
  stable?: boolean;
}

type Loader = "vanilla" | "forge" | "neoforge" | "fabric" | "optifine";

const LOADERS: { id: Loader; label: string; color: string }[] = [
  { id: "vanilla",  label: "Vanilla",  color: "#7CE38B" },
  { id: "forge",    label: "Forge",    color: "#FF8A4C" },
  { id: "neoforge", label: "NeoForge", color: "#A78BFA" },
  { id: "fabric",   label: "Fabric",   color: "#FFD27A" },
  { id: "optifine", label: "OptiFine", color: "#5EE9FF" },
];

interface CustomModpackPanelProps {
  onInstalled?: () => void | Promise<void>;
}

export default function CustomModpackPanel({ onInstalled }: CustomModpackPanelProps) {
  const [loader, setLoader] = useState<Loader>("forge");
  const [mcVersions, setMcVersions] = useState<McVersion[]>([]);
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersion[]>([]);
  const [selectedMc, setSelectedMc] = useState("");
  const [selectedLoader, setSelectedLoader] = useState("");
  const [modpackName, setModpackName] = useState("");
  const [ram, setRam] = useState(4096);
  const [jvmArgs, setJvmArgs] = useState("");
  const [showSnapshots, setShowSnapshots] = useState(false);

  const [loadingMc, setLoadingMc] = useState(false);
  const [loadingLoader, setLoadingLoader] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState(0);
  const [installMsg, setInstallMsg] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(false);


  useEffect(() => {
    loadMcVersions();
  }, []);


  useEffect(() => {
    if (selectedMc && loader !== "vanilla") {
      loadLoaderVersions();
    } else {
      setLoaderVersions([]);
      setSelectedLoader("");
    }
  }, [selectedMc, loader]);

  const loadMcVersions = async () => {
    setLoadingMc(true);
    setError("");
    try {
      const versions = await invoke<McVersion[]>("get_mc_versions");
      setMcVersions(versions);
      const first = versions.find(v => v.type === "release");
      if (first) setSelectedMc(first.id);
    } catch (e) {
      setError("Не удалось загрузить версии Minecraft: " + String(e));
    } finally {
      setLoadingMc(false);
    }
  };

  const loadLoaderVersions = async () => {
    setLoadingLoader(true);
    setSelectedLoader("");
    try {
      const versions = await invoke<LoaderVersion[]>("get_loader_versions", {
        loader,
        mcVersion: selectedMc,
      });
      setLoaderVersions(versions);
      if (versions.length > 0) setSelectedLoader(versions[0].id);
    } catch (e) {
      setLoaderVersions([]);
    } finally {
      setLoadingLoader(false);
    }
  };

  const handleInstall = async () => {
    if (!selectedMc) { setError("Выберите версию Minecraft"); return; }
    if (!modpackName.trim()) { setError("Введите название модпака"); return; }
    if (loader !== "vanilla" && !selectedLoader) { setError("Выберите версию загрузчика"); return; }

    setInstalling(true);
    setError("");
    setSuccess(false);
    setInstallProgress(0);
    setInstallMsg("Подготовка...");

    try {
      await invoke("install_custom_modpack", {
        name: modpackName.trim(),
        loader,
        mcVersion: selectedMc,
        loaderVersion: selectedLoader,
        maxMemory: ram,
        jvmArgs,
      });
      setInstallProgress(100);
      setInstallMsg("Готово!");
      setSuccess(true);
      await onInstalled?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setInstalling(false);
    }
  };

  const handleCancel = async () => {
    try {
      await invoke("cancel_download");
    } catch 
    setInstalling(false);
    setInstallProgress(0);
    setInstallMsg("");
  };

  const visibleMc = mcVersions.filter(v => showSnapshots || v.type === "release");

  return (
    <motion.div
      className="game-panel has-bg"
      key="custom"
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -15 }}
      transition={{ duration: 0.35, ease: "easeOut" }}
    >
      
      <div
        className="panel-bg"
        style={{ backgroundImage: "url(/backgrounds/custom.jpg)" }}
      />

      <div className="custom-panel">
        <h2>Свой модпак</h2>

        
        <div className="custom-card">
          <div className="custom-card-title">Загрузчик</div>
          <div className="loader-grid">
            {LOADERS.map(l => (
              <motion.div
                key={l.id}
                className={`loader-card ${loader === l.id ? "selected" : ""}`}
                onClick={() => { setLoader(l.id); setError(""); }}
                whileHover={{ scale: 1.04 }}
                whileTap={{ scale: 0.97 }}
                style={loader === l.id
                  ? {
                      borderColor: l.color,
                      boxShadow: `0 0 0 1px ${l.color}66, 0 8px 28px ${l.color}33`,
                      background: `linear-gradient(135deg, ${l.color}26, rgba(255,255,255,0.06))`,
                    }
                  : { borderColor: `${l.color}40` }}
              >
                <div className="loader-label" style={{ color: l.color }}>{l.label}</div>
              </motion.div>
            ))}
          </div>
        </div>

        
        <div className="custom-card">
          <div className="custom-card-title">Версия</div>
          <div className="version-selects">
          <div className="version-select-group">
            <label className="version-select-label">
              Minecraft
              <label className="snapshot-toggle">
                <input
                  type="checkbox"
                  checked={showSnapshots}
                  onChange={e => setShowSnapshots(e.target.checked)}
                />
                снапшоты
              </label>
            </label>
            {loadingMc ? (
              <div style={{ color: "rgba(255,255,255,0.5)", fontSize: 12, padding: "8px 0" }}>Загрузка...</div>
            ) : (
              <select
                className="version-select"
                value={selectedMc}
                onChange={e => setSelectedMc(e.target.value)}
              >
                {visibleMc.map(v => (
                  <option key={v.id} value={v.id}>{v.id}{v.type !== "release" ? ` (${v.type})` : ""}</option>
                ))}
              </select>
            )}
          </div>

          {loader !== "vanilla" && (
            <div className="version-select-group">
              <label className="version-select-label">{LOADERS.find(l2 => l2.id === loader)?.label} версия</label>
              {loadingLoader ? (
                <div style={{ color: "rgba(255,255,255,0.5)", fontSize: 12, padding: "8px 0" }}>Загрузка...</div>
              ) : loaderVersions.length === 0 ? (
                <div style={{ color: "rgba(255,200,100,0.8)", fontSize: 11, padding: "8px 0" }}>
                  {selectedMc ? "Нет версий для этого MC" : "Выберите версию MC"}
                </div>
              ) : (
                <select
                  className="version-select"
                  value={selectedLoader}
                  onChange={e => setSelectedLoader(e.target.value)}
                >
                  {loaderVersions.map(v => (
                    <option key={v.id} value={v.id}>
                      {v.id}{v.stable === false ? " (unstable)" : ""}
                    </option>
                  ))}
                </select>
              )}
            </div>
          )}
          </div>
        </div>

        
        <div className="custom-card">
          <div className="custom-card-title">Название модпака</div>
          <input
            className="modpack-name-input"
            placeholder="Например: MyPack 1.20.1"
            value={modpackName}
            onChange={e => { setModpackName(e.target.value); setError(""); }}
            maxLength={64}
          />
        </div>

        
        <div className="custom-card">
          <div className="custom-card-title">Память и JVM</div>
          <div className="ram-row">
            <input
              type="range" min={1024} max={16384} step={512} value={ram}
              onChange={e => setRam(Number(e.target.value))}
              className="ram-slider"
              style={{ "--slider-pct": ((ram - 1024) / (16384 - 1024) * 100) + "%" } as any}
            />
            <span className="ram-chip">
              {(ram / 1024).toFixed(1)} ГБ
            </span>
          </div>
          <textarea
            className="modpack-name-input modpack-textarea"
            placeholder="Дополнительные JVM-аргументы (необязательно)"
            value={jvmArgs}
            onChange={e => setJvmArgs(e.target.value)}
          />
        </div>

        
        <AnimatePresence>
          {error && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              style={{
                color: "#ff6b6b", fontSize: 12, marginBottom: 12,
                padding: "8px 12px",
                background: "rgba(255,80,80,0.12)",
                borderRadius: 6,
                border: "1px solid rgba(255,80,80,0.25)",
              }}
            >
              {error}
            </motion.div>
          )}
        </AnimatePresence>

        
        <AnimatePresence>
          {success && (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0 }}
              style={{
                color: "#7fff7f", fontSize: 13, marginBottom: 12, textAlign: "center",
                padding: "10px 14px",
                background: "rgba(57,255,20,0.1)",
                borderRadius: 6,
                border: "1px solid rgba(57,255,20,0.25)",
                fontWeight: 600,
              }}
            >
              Модпак «{modpackName}» установлен!
            </motion.div>
          )}
        </AnimatePresence>

        
        {installing && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            style={{ marginBottom: 14 }}
          >
            <div className="progress-bar-wrapper" style={{ background: "rgba(255,255,255,0.1)" }}>
              <motion.div
                className="progress-bar-fill"
                initial={{ width: "0%" }}
                animate={{ width: `${installProgress}%` }}
                transition={{ duration: 0.4 }}
              />
            </div>
            <div style={{ fontSize: 11, color: "rgba(255,255,255,0.6)", marginTop: 6 }}>{installMsg}</div>
          </motion.div>
        )}

        
        <div style={{ display: "flex", gap: 10 }}>
          {!installing ? (
            <motion.button
              className="play-button"
              style={{ flex: 1, letterSpacing: 1, backdropFilter: "blur(4px)", WebkitBackdropFilter: "blur(4px)" }}
              onClick={handleInstall}
              whileHover={{ scale: 1.03 }}
              whileTap={{ scale: 0.97 }}
            >
              Создать и установить
            </motion.button>
          ) : (
            <motion.button
              className="cancel-button"
              style={{
                flex: 1, borderColor: "rgba(255,100,100,0.6)",
                color: "#ff6b6b", background: "rgba(0,0,0,0.25)",
                backdropFilter: "blur(4px)",
              }}
              onClick={handleCancel}
              whileHover={{ scale: 1.03 }}
              whileTap={{ scale: 0.97 }}
            >
              Отменить
            </motion.button>
          )}
        </div>
      </div>
    </motion.div>
  );
}