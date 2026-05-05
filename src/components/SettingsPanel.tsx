import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
import { motion, AnimatePresence } from "framer-motion";

type Theme = "light" | "dark";
type GpuMode = "auto" | "discrete" | "integrated";

interface Props {
  javaPath: string;
  javaVersion: string;
  maxMemory: number;
  jvmArgs: string;
  gpuMode: string;
  theme: Theme;
  avatarUrl: string | null;
  username: string;
  onJavaChange: (path: string, version: string) => void;
  onMemoryChange: (mem: number) => void;
  onJvmArgsChange: (args: string) => void;
  onGpuModeChange: (mode: string) => void;
  allowMultipleInstances: boolean;
  closeLauncherOnGameStart: boolean;
  reopenLauncherAfterGameClose: boolean;
  onAllowMultipleInstancesChange: (value: boolean) => void;
  onCloseLauncherOnGameStartChange: (value: boolean) => void;
  onReopenLauncherAfterGameCloseChange: (value: boolean) => void;
  onThemeChange: (theme: Theme) => void;
  onAvatarChange: (url: string) => void;
}

interface JavaInfo {
  path: string;
  version: string;
  found: boolean;
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

export default function SettingsPanel({
  javaPath,
  javaVersion,
  maxMemory,
  jvmArgs,
  gpuMode,
  theme,
  avatarUrl,
  username,
  onJavaChange,
  onMemoryChange,
  onJvmArgsChange,
  onGpuModeChange,
  allowMultipleInstances,
  closeLauncherOnGameStart,
  reopenLauncherAfterGameClose,
  onAllowMultipleInstancesChange,
  onCloseLauncherOnGameStartChange,
  onReopenLauncherAfterGameCloseChange,
  onThemeChange,
  onAvatarChange,
}: Props) {
  const [javaStatus, setJavaStatus] = useState("");
  const [javaLoading, setJavaLoading] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateLoading, setUpdateLoading] = useState(false);
  const [updateMsg, setUpdateMsg] = useState("");
  const [avatarLoading, setAvatarLoading] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [toast, setToast] = useState("");
  const [loggingEnabled, setLoggingEnabled] = useState(() => {
    return localStorage.getItem("rpw_logging") !== "false";
  });
  const [logContent, setLogContent] = useState("");
  const [showLog, setShowLog] = useState(false);
  const [logPath, setLogPath] = useState("");
  const [buildDownloadDir, setBuildDownloadDir] = useState("");

  useEffect(() => {
    invoke<string>("get_build_download_dir")
      .then(setBuildDownloadDir)
      .catch(() => {});
  }, []);

  const showToast = (msg: string) => {
    setToast(msg);
    setTimeout(() => setToast(""), 3000);
  };

  const handleChooseBuildDownloadDir = async () => {
    try {
      const selected = await dialogOpen({ directory: true, multiple: false, title: "Выберите папку сохранения сборок" });
      if (typeof selected === "string") {
        await invoke("set_build_download_dir", { path: selected });
        setBuildDownloadDir(selected);
        showToast("Папка сохранения обновлена");
      }
    } catch (e) {
      showToast("Ошибка: " + String(e));
    }
  };


  const handleToggleLogging = async (enabled: boolean) => {
    setLoggingEnabled(enabled);
    localStorage.setItem("rpw_logging", enabled ? "true" : "false");
    try { await invoke("set_logging_enabled", { enabled }); } catch 
  };

  const handleViewLog = async () => {
    try {
      const [content, path] = await Promise.all([
        invoke<string>("get_log"),
        invoke<string>("get_log_path"),
      ]);
      setLogContent(content || "(лог пустой)");
      setLogPath(path);
      setShowLog(true);
    } catch (e) {
      showToast("Ошибка: " + String(e));
    }
  };

  const handleClearLog = async () => {
    try { await invoke("clear_log"); setLogContent("(лог пустой)"); showToast("Лог очищен"); } catch 
  };


  const handleFindJava = async () => {
    setJavaLoading(true);
    setJavaStatus("");
    try {
      const info = await invoke<JavaInfo>("find_java");
      if (info.found) {
        onJavaChange(info.path, info.version);
        setJavaStatus("Найдено: " + info.version);
      } else {
        setJavaStatus("Java не найдена");
      }
    } catch (e) {
      setJavaStatus("Ошибка: " + String(e));
    } finally {
      setJavaLoading(false);
    }
  };

  const handleDownloadJava = async () => {
    setJavaLoading(true);
    setJavaStatus("Скачивание Java 17...");
    try {
      const info = await invoke<JavaInfo>("download_java");
      onJavaChange(info.path, info.version);
      setJavaStatus("Java 17 скачана: " + info.version);
    } catch (e) {
      setJavaStatus("Ошибка: " + String(e));
    } finally {
      setJavaLoading(false);
    }
  };

  const handleBrowseJava = async () => {
    try {
      const selected = await dialogOpen({
        filters: [{ name: "Java", extensions: ["exe"] }],
        title: "Выберите java.exe",
      });
      if (typeof selected === "string") {
        onJavaChange(selected, "Ручной выбор");
        setJavaStatus("Путь установлен вручную");
      }
    } catch (e) {
      setJavaStatus("Ошибка: " + String(e));
    }
  };


  const handleAvatarClick = async () => {
    try {
      const selected = await dialogOpen({
        filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "gif", "webp"] }],
        title: "Выберите аватарку",
      });
      if (typeof selected === "string") {
        setAvatarLoading(true);
        try {
          const dataUrl = await invoke<string>("save_avatar", { sourcePath: selected });
          onAvatarChange(dataUrl);
          showToast("Аватарка обновлена");
        } catch (e) {
          showToast("Ошибка: " + String(e));
        } finally {
          setAvatarLoading(false);
        }
      }
    } catch 
  };


  const handleCheckUpdate = async () => {
    setUpdateLoading(true);
    setUpdateMsg("");
    setUpdateInfo(null);
    try {
      const info = await invoke<UpdateInfo>("check_launcher_update");
      setUpdateInfo(info);
      if (!info.update_available) setUpdateMsg(`Актуальная версия v${info.current_version}`);
    } catch (e) {
      setUpdateMsg("Ошибка проверки: " + String(e));
    } finally {
      setUpdateLoading(false);
    }
  };

  const handleDoUpdate = async () => {
    if (!updateInfo) return;
    setUpdateLoading(true);
    setUpdateMsg("Скачивание обновления...");
    try {
      await invoke("update_launcher", {
        downloadUrl: updateInfo.download_url,
        installerUrl: updateInfo.installer_url,
        isBareExe: false,
      });
      setUpdateMsg("Обновление запущено, лаунчер перезапустится...");
    } catch (e) {
      setUpdateMsg("Ошибка: " + String(e));
    } finally {
      setUpdateLoading(false);
    }
  };


  const handleOpenDataFolder = async () => {
    try {
      await invoke("open_data_folder");
    } catch (e) {
      showToast("Ошибка: " + String(e));
    }
  };


  const handleDeleteLauncher = async () => {
    if (!deleteConfirm) {
      setDeleteConfirm(true);
      setTimeout(() => setDeleteConfirm(false), 4000);
      return;
    }
    try {
      await invoke("delete_launcher");
    } catch (e) {
      showToast("Ошибка: " + String(e));
    }
  };

  return (
    <div className="settings-panel">
      <h2 style={{ marginBottom: 24, fontWeight: 700, fontSize: 20 }}>Настройки</h2>

      
      <Section title="Профиль">
        <div style={{ display: "flex", gap: 18, alignItems: "center" }}>
          <div
            className="avatar-wrapper"
            onClick={handleAvatarClick}
            title="Нажмите чтобы сменить аватарку"
            style={{ cursor: "pointer" }}
          >
            {avatarLoading ? (
              <div className="avatar-placeholder" style={{ fontSize: 20 }}>...</div>
            ) : avatarUrl ? (
              <img src={avatarUrl} alt="avatar" className="avatar-img" />
            ) : (
              <div className="avatar-placeholder">{username[0]?.toUpperCase()}</div>
            )}
            <div className="avatar-overlay">
              <svg viewBox="0 0 16 16" fill="none" width="12" height="12">
                <path d="M11 2L14 5L5 14H2V11L11 2Z" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round"/>
              </svg>
            </div>
          </div>
          <div>
            <div style={{ fontWeight: 600, fontSize: 15 }}>{username}</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 3 }}>
              Нажмите на аватарку чтобы сменить (png, jpg, gif, webp)
            </div>
          </div>
        </div>
      </Section>

      
      <Section title="Оформление">
        <div style={{ display: "flex", gap: 10 }}>
          {(["light", "dark"] as Theme[]).map(t => (
            <motion.button
              key={t}
              className={`theme-btn ${theme === t ? "active" : ""}`}
              onClick={() => onThemeChange(t)}
              whileHover={{ scale: 1.04 }}
              whileTap={{ scale: 0.96 }}
            >
              {t === "light" ? "Светлая" : "Тёмная"}
            </motion.button>
          ))}
        </div>
      </Section>

      
      <Section title="Память (RAM)">
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <input
            type="range" min={1024} max={16384} step={512}
            value={maxMemory}
            onChange={e => onMemoryChange(Number(e.target.value))}
            className="ram-slider"
            style={{ flex: 1, "--slider-pct": ((maxMemory - 1024) / (16384 - 1024) * 100) + "%" } as React.CSSProperties & { "--slider-pct": string }}
          />
          <span style={{ fontWeight: 700, color: "var(--accent)", minWidth: 56, fontSize: 14 }}>
            {(maxMemory / 1024).toFixed(1)} ГБ
          </span>
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 5 }}>
          Рекомендуется: 4–8 ГБ для модпаков, 2 ГБ для ванилы
        </div>
      </Section>

      
      <Section title="Java">
        <div style={{ fontSize: 12, marginBottom: 8, color: "var(--text-2)" }}>
          {javaPath
            ? <><span style={{ color: "var(--accent)" }}>OK</span> {javaVersion || "Выбран"}<br /><span style={{ opacity: 0.55, fontSize: 10 }}>{javaPath}</span></>
            : <span style={{ color: "var(--accent-2)" }}>Java не выбрана</span>
          }
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Btn onClick={handleFindJava} loading={javaLoading}>Найти</Btn>
          <Btn onClick={handleDownloadJava} loading={javaLoading}>Скачать Java 17</Btn>
          <Btn onClick={handleBrowseJava}>Обзор</Btn>
        </div>
        {javaStatus && (
          <div style={{ fontSize: 11, marginTop: 8, color: "var(--text-muted)" }}>
            {javaStatus}
          </div>
        )}
      </Section>

      
      <Section title="JVM аргументы">
        <textarea
          className="jvm-args-input"
          placeholder="-XX:+UseG1GC -XX:MaxGCPauseMillis=50 ..."
          value={jvmArgs}
          onChange={e => onJvmArgsChange(e.target.value)}
          rows={3}
        />
        <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 5 }}>
          Оставьте пустым для значений по умолчанию (G1GC, рекомендуется)
        </div>
      </Section>

      
      <Section title="GPU / Видеокарта">
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {([
            { id: "auto",       label: "Авто" },
            { id: "discrete",   label: "Дискретная (NVIDIA/AMD)" },
            { id: "integrated", label: "Встроенная" },
          ] as { id: GpuMode; label: string }[]).map(opt => (
            <motion.button
              key={opt.id}
              className={`gpu-btn ${gpuMode === opt.id ? "active" : ""}`}
              onClick={() => onGpuModeChange(opt.id)}
              whileHover={{ scale: 1.04 }}
              whileTap={{ scale: 0.96 }}
            >
              {opt.label}
            </motion.button>
          ))}
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6 }}>
          «Дискретная» передаёт NVIDIA Optimus / AMD MXM аргументы для использования игровой карты
        </div>
      </Section>

      
      <Section title="Запуск Minecraft">
        <SettingToggle
          checked={allowMultipleInstances}
          onChange={onAllowMultipleInstancesChange}
          title="Разрешить твинки"
          description="Если выключено, кнопка «Играть» блокируется пока уже открыт клиент Minecraft."
        />
        <SettingToggle
          checked={closeLauncherOnGameStart}
          onChange={onCloseLauncherOnGameStartChange}
          title="Закрывать лаунчер при запуске Minecraft"
          description="По умолчанию включено: лаунчер закрывается после успешного запуска игры."
        />
        <SettingToggle
          checked={reopenLauncherAfterGameClose}
          onChange={onReopenLauncherAfterGameCloseChange}
          title="Открывать лаунчер после закрытия Minecraft"
          description="Работает если включено закрытие лаунчера при запуске игры."
        />
      </Section>

      
      <Section title="Данные лаунчера">
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 10 }}>
          Все данные хранятся в: <code style={{ opacity: 0.8 }}>%APPDATA%\.rpworld</code>
        </div>
        <Btn onClick={handleOpenDataFolder}>Открыть папку данных</Btn>
      </Section>

      <Section title="Скачивание сборок">
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 10, wordBreak: "break-all" }}>
          Папка сохранения модов и сборок: <code style={{ opacity: 0.9 }}>{buildDownloadDir || "не выбрана"}</code>
        </div>
        <Btn onClick={handleChooseBuildDownloadDir}>Изменить папку сохранения</Btn>
      </Section>

      
      <Section title="Обновление лаунчера">
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <Btn onClick={handleCheckUpdate} loading={updateLoading}>
            Проверить обновления
          </Btn>
          {updateInfo?.update_available && (
            <Btn onClick={handleDoUpdate} loading={updateLoading} accent>
              Обновить до v{updateInfo.latest_version}
            </Btn>
          )}
        </div>
        {updateMsg && (
          <div style={{ fontSize: 12, marginTop: 8, color: "var(--text-2)" }}>
            {updateMsg}
          </div>
        )}
        {updateInfo?.update_available && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            style={{
              marginTop: 10,
              padding: "10px 12px",
              background: "var(--bg-overlay)",
              border: "1px solid var(--border)",
              borderRadius: 8,
              fontSize: 12,
            }}
          >
            <strong>v{updateInfo.latest_version}</strong> — {(updateInfo.file_size / 1024 / 1024).toFixed(1)} МБ
            {updateInfo.release_notes && (
              <div style={{ marginTop: 6, opacity: 0.75, whiteSpace: "pre-line" }}>
                {updateInfo.release_notes.slice(0, 300)}
              </div>
            )}
          </motion.div>
        )}
      </Section>

      
      <Section title="Удаление лаунчера" danger>
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 10 }}>
          Удалит все данные лаунчера, модпаки и Java из <code>%APPDATA%\.rpworld</code>, затем запустит деинсталляцию.
        </div>
        <motion.button
          className={`delete-btn ${deleteConfirm ? "confirm" : ""}`}
          onClick={handleDeleteLauncher}
          whileHover={{ scale: 1.03 }}
          whileTap={{ scale: 0.96 }}
        >
          {deleteConfirm ? "Нажмите ещё раз для подтверждения" : "Удалить лаунчер"}
        </motion.button>
      </Section>

      
      <Section title="Логи лаунчера">
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer", fontSize: 13 }}>
            <input
              type="checkbox"
              checked={loggingEnabled}
              onChange={e => handleToggleLogging(e.target.checked)}
              style={{ width: 15, height: 15, accentColor: "var(--accent)", cursor: "pointer" }}
            />
            Вести лог лаунчера
          </label>
        </div>
        {loggingEnabled && (
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Btn onClick={handleViewLog}>Открыть лог</Btn>
            <Btn onClick={handleClearLog}>Очистить лог</Btn>
          </div>
        )}
        {showLog && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            style={{ marginTop: 10 }}
          >
            {logPath && (
              <div style={{ fontSize: 10, color: "var(--text-muted)", marginBottom: 6, opacity: 0.7 }}>
                {logPath}
              </div>
            )}
            <textarea
              readOnly
              value={logContent}
              style={{
                width: "100%", height: 180, resize: "vertical",
                fontSize: 10, fontFamily: "'Cascadia Code','Fira Code',monospace",
                padding: "8px 10px", borderRadius: "var(--r-sm)",
                background: "var(--bg-glass)", border: "1px solid var(--border)",
                color: "var(--text)", outline: "none",
              }}
            />
          </motion.div>
        )}
      </Section>

      
      <AnimatePresence>
        {toast && (
          <motion.div
            className="notification"
            style={{ position: "fixed", bottom: 20, right: 20, zIndex: 999 }}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 12 }}
          >
            {toast}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function Section({
  title,
  children,
  danger,
}: {
  title: string;
  children: React.ReactNode;
  danger?: boolean;
}) {
  return (
    <motion.div
      className={`settings-section ${danger ? "danger" : ""}`}
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25 }}
    >
      <div className="settings-section-title">{title}</div>
      {children}
    </motion.div>
  );
}

function SettingToggle({
  checked,
  onChange,
  title,
  description,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  title: string;
  description: string;
}) {
  return (
    <label className="setting-toggle-row">
      <input
        type="checkbox"
        checked={checked}
        onChange={e => onChange(e.target.checked)}
      />
      <span className="setting-toggle-box" />
      <span className="setting-toggle-copy">
        <span className="setting-toggle-title">{title}</span>
        <span className="setting-toggle-desc">{description}</span>
      </span>
    </label>
  );
}

function Btn({
  children,
  onClick,
  loading,
  accent,
}: {
  children: React.ReactNode;
  onClick?: () => void;
  loading?: boolean;
  accent?: boolean;
}) {
  return (
    <motion.button
      className={`settings-btn ${accent ? "accent" : ""}`}
      onClick={onClick}
      disabled={loading}
      whileHover={{ scale: 1.04 }}
      whileTap={{ scale: 0.96 }}
    >
      {loading ? "..." : children}
    </motion.button>
  );
}