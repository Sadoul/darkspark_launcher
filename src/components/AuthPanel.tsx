import { useState } from "react";
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

const ADMIN_NAME = "DarkSpark00";
const ADMIN_PASSWORD = "Oiw$8z09o@H8";

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

const LockIcon = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
    <rect x="5" y="11" width="14" height="10" rx="2" />
    <path d="M8 11V7a4 4 0 0 1 8 0v4" />
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

const LAST_USERNAME_KEY = "darkspark_last_username";

export default function AuthPanel({ onLogin }: AuthPanelProps) {
  const [username, setUsername] = useState(() => localStorage.getItem(LAST_USERNAME_KEY) ?? "");
  const [password, setPassword] = useState("");
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [pendingUsername, setPendingUsername] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleLogin = async () => {
    const cleanUsername = username.trim();

    if (!cleanUsername || cleanUsername.length < 3) {
      setError("Никнейм должен быть минимум 3 символа");
      return;
    }
    if (cleanUsername.length > 16) {
      setError("Никнейм не может быть длиннее 16 символов");
      return;
    }
    if (!/^[a-zA-Z0-9_]+$/.test(cleanUsername)) {
      setError("Только латинские буквы, цифры и _");
      return;
    }

    // Admin login — DarkSpark00 requires password
    if (cleanUsername.toLowerCase() === ADMIN_NAME.toLowerCase()) {
      if (!showPasswordPrompt) {
        setPendingUsername(cleanUsername);
        setShowPasswordPrompt(true);
        setPassword("");
        setError("");
        return;
      }

      if (password !== ADMIN_PASSWORD) {
        setError("Неверный пароль");
        return;
      }

      setLoading(true);
      setError("");
      try {
        const account = await invoke<Account>("login_darkspark", {
          username: pendingUsername,
          password,
        });
        localStorage.setItem(LAST_USERNAME_KEY, pendingUsername);
        onLogin(account);
      } catch (err) {
        setError(String(err));
      } finally {
        setLoading(false);
      }
      return;
    }

    // Regular offline login
    if (showPasswordPrompt) {
      setShowPasswordPrompt(false);
      setPassword("");
    }
    setLoading(true);
    setError("");
    try {
      const account = await invoke<Account>("login_offline", { username: cleanUsername });
      localStorage.setItem(LAST_USERNAME_KEY, cleanUsername);
      onLogin(account);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleBack = () => {
    setShowPasswordPrompt(false);
    setPassword("");
    setPendingUsername("");
    setError("");
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
            <img src="/icons/Inside.png" alt="DanganVerse" className="auth-modal-logo" draggable={false} />
            <div className="auth-modal-logo-glow" />
          </div>
          <h1 className="auth-modal-title">DanganVerse Launcher</h1>
          {showPasswordPrompt ? (
            <p className="auth-modal-subtitle">Введите пароль администратора</p>
          ) : (
            <p className="auth-modal-subtitle">Введите никнейм для входа</p>
          )}
        </motion.div>

        <motion.div className="auth-modal-form" custom={1} variants={itemVariants} initial="hidden" animate="visible">
          <div className="auth-form-inner">
            <div className="auth-input-wrap">
              <div className="auth-input-icon">
                {showPasswordPrompt ? <LockIcon /> : <UserIcon />}
              </div>
              <input
                type={showPasswordPrompt ? "password" : "text"}
                className="auth-modal-input"
                placeholder={showPasswordPrompt ? "Введите пароль..." : "Введите никнейм..."}
                value={showPasswordPrompt ? password : username}
                onChange={(e) => showPasswordPrompt ? setPassword(e.target.value) : setUsername(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleLogin()}
                maxLength={showPasswordPrompt ? 64 : 16}
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
              disabled={loading}
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
                  {showPasswordPrompt ? "Войти" : "Войти"}
                </span>
              )}
            </motion.button>

            {showPasswordPrompt && (
              <motion.button
                className="auth-modal-back"
                onClick={handleBack}
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
              >
                Назад к вводу ника
              </motion.button>
            )}
          </div>
        </motion.div>

        <motion.div className="auth-modal-footer" custom={3} variants={itemVariants} initial="hidden" animate="visible">
          <div className="auth-modal-divider" />
          <span className="auth-modal-version">DanganVerse Launcher</span>
        </motion.div>
      </motion.div>
    </motion.div>
  );
}
