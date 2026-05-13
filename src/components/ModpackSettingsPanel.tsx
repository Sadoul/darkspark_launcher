import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Page } from "./Sidebar";
import type { CustomModpack } from "../App";

const BUILTIN_CONFIGS: Record<string, { title: string; loader: string; mcVersion: string; loaderVersion: string; memory: number; jvmArgs: string }> = {
  danganverse: { title: "DanganVerse", loader: "fabric", mcVersion: "1.20.1", loaderVersion: "", memory: 4096, jvmArgs: "" },
};

interface Props {
  page: Page;
  customModpacks: CustomModpack[];
  onBack: () => void;
  onChanged: () => void;
}

export default function ModpackSettingsPanel({ page, customModpacks, onBack, onChanged }: Props) {
  const isCustom = page.startsWith("custom:");
  const customName = isCustom ? page.slice("custom:".length) : "";
  const custom = customModpacks.find((pack) => pack.name === customName);
  const builtin = !isCustom ? BUILTIN_CONFIGS[page] : null;

  const [name, setName] = useState(custom?.name ?? builtin?.title ?? "Сборка");
  const [loader, setLoader] = useState(custom?.loader ?? builtin?.loader ?? "forge");
  const [mcVersion, setMcVersion] = useState(custom?.mc_version ?? builtin?.mcVersion ?? "1.20.1");
  const [loaderVersion, setLoaderVersion] = useState(custom?.loader_version ?? builtin?.loaderVersion ?? "");
  const [memory, setMemory] = useState(() => {
    if (!isCustom) {
      const saved = localStorage.getItem(`rpw_modpack_memory_${page}`);
      const parsed = saved ? Number(saved) : NaN;
      if (!Number.isNaN(parsed)) return parsed;
    }
    return custom?.max_memory ?? builtin?.memory ?? 4096;
  });
  const [jvmArgs, setJvmArgs] = useState(custom?.jvm_args ?? builtin?.jvmArgs ?? "");
  const [gameDir, setGameDir] = useState(custom?.game_dir ?? "");
  const [message, setMessage] = useState("");
  const [mcVersionOptions, setMcVersionOptions] = useState<string[]>([]);
  const [loaderVersionOptions, setLoaderVersionOptions] = useState<string[]>([]);
  const [loadingLoaderVersions, setLoadingLoaderVersions] = useState(false);

  useEffect(() => {
    if (!isCustom && page === "danganverse") {
      invoke<string>("get_builtin_modpack_dir", { modpackName: page })
        .then(setGameDir)
        .catch(() => {});
    }
  }, [isCustom, page]);

  useEffect(() => {
    if (!isCustom) return;
    invoke<Array<{ id: string; type?: string; version_type?: string }>>("get_mc_versions")
      .then((list) => {
        const releases = list.filter((v) => (v.type ?? v.version_type) === "release").map((v) => v.id);
        setMcVersionOptions(releases);
      })
      .catch(() => setMcVersionOptions([]));
  }, [isCustom]);

  useEffect(() => {
    if (!isCustom) return;
    if (loader === "vanilla") {
      setLoaderVersionOptions([]);
      setLoaderVersion("");
      return;
    }
    setLoadingLoaderVersions(true);
    invoke<Array<{ id: string }>>("get_loader_versions", { loader, mcVersion })
      .then((list) => {
        const ids = list.map((v) => v.id);
        setLoaderVersionOptions(ids);
        if (ids.length > 0 && !ids.includes(loaderVersion)) setLoaderVersion(ids[0]);
      })
      .catch(() => setLoaderVersionOptions([]))
      .finally(() => setLoadingLoaderVersions(false));

  }, [isCustom, loader, mcVersion]);

  const openFolder = async () => {
    try {
      if (isCustom && custom?.game_dir) {
        await invoke("open_path", { path: custom.game_dir });
      } else {
        await invoke("open_builtin_modpack_folder", { modpackName: page });
      }
    } catch (e) {
      setMessage(String(e));
    }
  };

  const save = async () => {
    if (!isCustom) {
      localStorage.setItem(`rpw_modpack_memory_${page}`, String(memory));
      setMessage("ОЗУ сборки сохранено локально");
      return;
    }
    try {
      await invoke("install_custom_modpack", {
        name,
        loader,
        mcVersion,
        loaderVersion,
        maxMemory: memory,
        jvmArgs,
        previousName: customName || undefined,
      });
      setMessage("Настройки модпака сохранены");
      onChanged();
    } catch (e) {
      setMessage(String(e));
    }
  };

  const deletePack = async () => {
    const title = isCustom ? name : "danganverse";
    if (!confirm(`Удалить установленную сборку «${title}» с компьютера? Лаунчер останется.`)) return;
    try {
      if (isCustom) {
        await invoke("delete_custom_modpack", { name: customName });
      } else {
        await invoke("delete_builtin_modpack", { modpackName: page });
      }
      setMessage("Сборка удалена с компьютера");
      onChanged();
    } catch (e) {
      setMessage(String(e));
    }
  };

  void page;

  const [editingMemory, setEditingMemory] = useState(false);
  const [memoryInput, setMemoryInput] = useState(String(memory));

  const handleMemoryInputBlur = () => {
    const val = parseInt(memoryInput);
    if (!isNaN(val)) setMemory(Math.max(1024, Math.min(32768, val)));
    setEditingMemory(false);
  };

  return (
    <div className="settings-panel modpack-settings-panel">
      <div className="settings-header">
        <button className="back-icon-btn" onClick={onBack} title="Назад">
          <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M12.5 4.5L7 10l5.5 5.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </button>
        <div style={{ flex: 1 }}>
          <h2>Настройки сборки</h2>
          <p>{name} · отдельные параметры модпака</p>
        </div>
      </div>

      <div className="modpack-settings-grid">
        {isCustom && (
          <div className="admin-card">
            <label>Название сборки</label>
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Название..." />
          </div>
        )}
        
        {isCustom && (
          <div className="admin-card">
            <label>Тип загрузчика</label>
            <select value={loader} onChange={(e) => setLoader(e.target.value)}>
              <option value="vanilla">Vanilla</option>
              <option value="forge">Forge</option>
              <option value="fabric">Fabric</option>
              <option value="neoforge">NeoForge</option>
              <option value="optifine">OptiFine</option>
            </select>
          </div>
        )}

        {isCustom && (
          <div className="admin-card">
            <label>Версия Minecraft</label>
            <select value={mcVersion} onChange={(e) => setMcVersion(e.target.value)}>
              {!mcVersionOptions.includes(mcVersion) && mcVersion && (
                <option value={mcVersion}>{mcVersion}</option>
              )}
              {mcVersionOptions.map((id) => (
                <option key={id} value={id}>{id}</option>
              ))}
            </select>
          </div>
        )}

        {isCustom && loader !== "vanilla" && (
          <div className="admin-card">
            <label>Версия загрузчика</label>
            <select value={loaderVersion} onChange={(e) => setLoaderVersion(e.target.value)} disabled={loadingLoaderVersions}>
              {loadingLoaderVersions && <option>Загрузка...</option>}
              {!loadingLoaderVersions && loaderVersion && !loaderVersionOptions.includes(loaderVersion) && (
                <option value={loaderVersion}>{loaderVersion}</option>
              )}
              {!loadingLoaderVersions && loaderVersionOptions.map((id) => (
                <option key={id} value={id}>{id}</option>
              ))}
            </select>
          </div>
        )}

        <div className="admin-card wide">
          <label>Выделение ОЗУ (МБ)</label>
          <div className="memory-row">
            <input type="range" min={1024} max={16384} step={512} value={memory}
              onChange={(e) => {
                const next = Number(e.target.value);
                setMemory(next);
                setMemoryInput(String(next));
              }}
              className="ram-slider"
              style={{ ["--slider-pct" as any]: `${Math.round(((memory - 1024) / (16384 - 1024)) * 100)}%` }}
            />

            {editingMemory ? (
              <input
                className="memory-input"
                autoFocus
                inputMode="numeric"
                size={Math.max(4, memoryInput.length + 2)}
                value={memoryInput}
                onChange={(e) => setMemoryInput(e.target.value)}
                onBlur={handleMemoryInputBlur}
                onKeyDown={(e) => e.key === "Enter" && handleMemoryInputBlur()}
              />
            ) : (
              <span className="memory-chip clickable" onClick={() => setEditingMemory(true)} title="Нажмите, чтобы ввести вручную">
                {memory} МБ ({(memory / 1024).toFixed(1)} ГБ)
              </span>
            )}
          </div>
        </div>

        {isCustom && (
          <div className="admin-card wide">
            <label>JVM аргументы</label>
            <textarea value={jvmArgs} onChange={(e) => setJvmArgs(e.target.value)} rows={4} placeholder="Например: -XX:+UseG1GC" />
          </div>
        )}

        <div className="admin-card wide">
          <label>Папка сборки</label>
          <div className="folder-row">
            <code>{gameDir || "Папка ещё не создана"}</code>
            <button className="folder-icon-btn" onClick={openFolder} title="Открыть папку сборки">
              <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M2 5.5A1.5 1.5 0 013.5 4h3.382a1.5 1.5 0 011.06.44l.94.94H16.5A1.5 1.5 0 0118 7v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 15V5.5z"
                  stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />
              </svg>
            </button>
          </div>
        </div>
      </div>

      <div className="modpack-settings-actions">
        <button className="settings-btn primary large-btn" onClick={save}>Сохранить настройки</button>
        <button className="settings-btn danger large-btn" onClick={deletePack}>Удалить сборку с компьютера</button>
      </div>
      {message && <div className="admin-message modpack-settings-message">{message}</div>}
    </div>
  );
}
