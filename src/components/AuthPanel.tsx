import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";

interface Account {
  username: string;
  uuid: string;
  access_token: string;
  account_type: string;
}

interface AuthPanelProps {
  onLogin: (account: Account) => void;
}

const UserIcon = () => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="8" r="4" />
    <path d="M5.5 21a6.5 6.5 0 0 1 13 0" />
  </svg>
);

const ArrowIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M5 12h14M12 5l7 7-7 7" />
  </svg>
);

const overlayVariants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: { duration: 0.4, ease: "easeOut" } },
};

const cardVariants = {
  hidden: { opacity: 0, y: 30, scale: 0.96 },
  visible: {
    opacity: 1, y: 0, scale: 1,
    transition: { duration: 0.5, ease: [0.16, 1, 0.3, 1], delay: 0.1 },
  },
};

const itemVariants = {
  hidden: { opacity: 0, y: 12 },
  visible: (i: number) => ({
    opacity: 1, y: 0,
    transition: { duration: 0.4, ease: "easeOut", delay: 0.2 + i * 0.08 },
  }),
};

export default function AuthPanel({ onLogin }: AuthPanelProps) {
  const [username, setUsername] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleLogin = async () => {
    if (!username.trim() || username.length < 3) {
      setError("Никнейм должен быть минимум 3 символа");
      return;
    }
    if (username.length > 16) {
      setError("Никнейм не может быть длиннее 16 символов");
      return;
    }
    if (!/^[a-zA-Z0-9_]+$/.test(username)) {
      setError("Только латинские буквы, цифры и _");
      return;
    }

    setLoading(true);
    setError("");
    try {
      const cleanUsername = username.trim();
      const account = await invoke<Account>("login_offline", { username: cleanUsername });
      onLogin(account);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <motion.div
      className="auth-modal-overlay"
      variants={overlayVariants}
      initial="hidden"
      animate="visible"
      exit="hidden"
    >
      <div className="auth-modal-orbs">
        <motion.div
          className="auth-orb auth-orb-1"
          animate={{ x: [0, 30, -20, 0], y: [0, -20, 10, 0] }}
          transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
        />
        <motion.div
          className="auth-orb auth-orb-2"
          animate={{ x: [0, -25, 15, 0], y: [0, 15, -25, 0] }}
          transition={{ duration: 25, repeat: Infinity, ease: "linear" }}
        />
        <motion.div
          className="auth-orb auth-orb-3"
          animate={{ x: [0, 18, -12, 0], y: [0, -30, 20, 0] }}
          transition={{ duration: 18, repeat: Infinity, ease: "linear" }}
        />
      </div>

      <motion.div
        className="auth-modal-card"
        variants={cardVariants}
        initial="hidden"
        animate="visible"
      >
        <div className="auth-modal-accent-line" />

        <motion.div className="auth-modal-header" custom={0} variants={itemVariants} initial="hidden" animate="visible">
          <div className="auth-modal-logo-wrap">
            <img src="/icons/Inside.png" alt="DarkSpark" className="auth-modal-logo" draggable={false} />
            <div className="auth-modal-logo-glow" />
          </div>
          <h1 className="auth-modal-title">DarkSpark Launcher</h1>
          <p className="auth-modal-subtitle">Введите никнейм для входа</p>
        </motion.div>

        <motion.div className="auth-modal-form" custom={1} variants={itemVariants} initial="hidden" animate="visible">
          <div className="auth-form-inner">
            <div className="auth-input-wrap">
              <div className="auth-input-icon">
                <UserIcon />
              </div>
              <input
                type="text"
                className="auth-modal-input"
                placeholder="Введите никнейм..."
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleLogin()}
                maxLength={16}
                autoFocus
              />
            </div>

            <AnimatePresence>
              {error && (
                <motion.div
                  className="auth-modal-error"
                  initial={{ opacity: 0, y: -4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -4 }}
                >
                  {error}
                </motion.div>
              )}
            </AnimatePresence>

            <motion.button
              className="auth-modal-submit"
              onClick={handleLogin}
              disabled={loading || !username.trim()}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
            >
              {loading ? (
                <span className="auth-modal-loading">
                  <div className="spinner" style={{ width: 16, height: 16, borderWidth: 2 }} />
                  Вход...
                </span>
              ) : (
                <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <ArrowIcon />
                  Войти
                </span>
              )}
            </motion.button>
          </div>
        </motion.div>

        <motion.div className="auth-modal-footer" custom={3} variants={itemVariants} initial="hidden" animate="visible">
          <div className="auth-modal-divider" />
          <span className="auth-modal-version">DarkSpark Launcher</span>
        </motion.div>
      </motion.div>
    </motion.div>
  );
}
