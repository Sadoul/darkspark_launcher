import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { motion } from "framer-motion";

interface TitlebarProps {
  theme: "light" | "dark";
  onThemeToggle: () => void;
}

export default function Titlebar({ theme, onThemeToggle }: TitlebarProps) {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    win.isMaximized().then(setIsMaximized);
    const unlisten = win.onResized(() => {
      win.isMaximized().then(setIsMaximized);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const handleMinimize = () => getCurrentWindow().minimize();
  const handleMaximize = () => getCurrentWindow().toggleMaximize();
  const handleClose = () => getCurrentWindow().close();

  return (
    <div className="titlebar" data-tauri-drag-region>
      {}
      <div className="titlebar-brand" data-tauri-drag-region />

      {}
      <div className="titlebar-buttons">
        {}
        <motion.button
          className="titlebar-btn theme-toggle"
          onClick={onThemeToggle}
          title={theme === "dark" ? "Светлая тема" : "Тёмная тема"}
          whileHover={{ scale: 1.1 }}
          whileTap={{ scale: 0.9 }}
        >
          {theme === "dark" ? (
            
            <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
              <circle cx="8" cy="8" r="3" stroke="currentColor" strokeWidth="1.4"/>
              <line x1="8" y1="1.5" x2="8" y2="2.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <line x1="8" y1="13.5" x2="8" y2="14.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <line x1="1.5" y1="8" x2="2.5" y2="8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <line x1="13.5" y1="8" x2="14.5" y2="8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <line x1="3.4" y1="3.4" x2="4.1" y2="4.1" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <line x1="11.9" y1="11.9" x2="12.6" y2="12.6" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <line x1="12.6" y1="3.4" x2="11.9" y2="4.1" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
              <line x1="4.1" y1="11.9" x2="3.4" y2="12.6" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
            </svg>
          ) : (
            
            <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M13.5 9.5A5.5 5.5 0 016.5 2.5a5.5 5.5 0 100 11 5.5 5.5 0 007-4z" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
            </svg>
          )}
        </motion.button>

        {}
        <motion.button
          className="titlebar-btn"
          whileHover={{ scale: 1.1 }}
          whileTap={{ scale: 0.9 }}
          onClick={handleMinimize}
          title="Свернуть"
        >
          <svg viewBox="0 0 16 16" fill="none">
            <line x1="3" y1="8" x2="13" y2="8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
          </svg>
        </motion.button>

        {}
        <motion.button
          className="titlebar-btn"
          whileHover={{ scale: 1.1 }}
          whileTap={{ scale: 0.9 }}
          onClick={handleMaximize}
          title={isMaximized ? "Восстановить" : "Развернуть"}
        >
          {isMaximized ? (
            <svg viewBox="0 0 16 16" fill="none">
              <rect x="5.5" y="3" width="7.5" height="7.5" rx="1" stroke="currentColor" strokeWidth="1.4"/>
              <path d="M3 5.5V12a1 1 0 001 1h6.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
            </svg>
          ) : (
            <svg viewBox="0 0 16 16" fill="none">
              <rect x="3" y="3" width="10" height="10" rx="1.5" stroke="currentColor" strokeWidth="1.4"/>
            </svg>
          )}
        </motion.button>

        {}
        <motion.button
          className="titlebar-btn close"
          whileHover={{ scale: 1.05, backgroundColor: "#ef4444" }}
          whileTap={{ scale: 0.9 }}
          onClick={handleClose}
          title="Закрыть"
        >
          <svg viewBox="0 0 16 16" fill="none">
            <line x1="4" y1="4" x2="12" y2="12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
            <line x1="12" y1="4" x2="4" y2="12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
          </svg>
        </motion.button>
      </div>
    </div>
  );
}