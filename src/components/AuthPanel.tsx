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

const MicrosoftIcon = () => (
  <svg width="20" height="20" viewBox="0 0 21 21" fill="none">
    <rect x="1" y="1" width="9" height="9" rx="1" fill="#F25022" />
    <rect x="11" y="1" width="9" height="9" rx="1" fill="#7FBA00" />
    <rect x="1" y="11" width="9" height="9" rx="1" fill="#00A4EF" />
    <rect x="11" y="11" width="9" height="9" rx="1" fill="#FFB900" />
  </svg>
);

const ShieldIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2l7.5 4v5.5c0 5-3 8.5-7.5 10.5-4.5-2-7.5-5.5-7.5-10.5V6L12 2z" />
    <path d="M9 12l2 2 4-4" />
  </svg>
);

const OfflineWifiOffIcon = () => (
  <svg width="28" height="28" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
    <path d="M2 8.5C4.7 6.3 8.2 5 12 5c3.8 0 7.3 1.3 10 3.5" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" opacity="0.9"/>
    <path d="M5.5 12c1.8-1.4 4-2.2 6.5-2.2 2.5 0 4.7.8 6.5 2.2" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" opacity="0.75"/>
    <path d="M9 15.5c.9-.6 1.9-.9 3-.9s2.1.3 3 .9" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" opacity="0.65"/>
    <circle cx="12" cy="19" r="1.35" fill="currentColor"/>
    <path d="M4 4l16 16" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"/>
  </svg>
);

const RpworldAccountIcon = () => (
  <svg width="28" height="28" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
    <rect x="3" y="4" width="18" height="16" rx="5" stroke="currentColor" strokeWidth="1.8" />
    <circle cx="9" cy="11" r="2.2" stroke="currentColor" strokeWidth="1.7" />
    <path d="M5.8 18c.8-2.2 2-3.4 3.2-3.4s2.4 1.2 3.2 3.4" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
    <path d="M15 9.2h3.2M15 12h2.4M15 14.8h3.2" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
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
  const [selectedMethod, setSelectedMethod] = useState<"offline" | "rpworld" | "microsoft" | null>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [saveProfile, setSaveProfile] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<{ username: string; password: string } | null>("get_saved_offline_profile")
      .then(profile => {
        if (!profile) return;
        setUsername(profile.username);
        setPassword(profile.password);
        setSaveProfile(true);
        setSelectedMethod("rpworld");
      })
      .catch(() => {});
  }, []);

  const handleOfflineLogin = async () => {
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

  const handleRpworldLogin = async () => {
    if (!username.trim() || username.length < 3) {
      setError("Никнейм должен быть минимум 3 символа");
      return;
    }
    if (!password.trim()) {
      setError("Введите пароль RPWorld аккаунта");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const cleanUsername = username.trim();
      const account = await invoke<Account>("login_rpworld", { username: cleanUsername, password });
      if (saveProfile) {
        await invoke("save_offline_profile", { username: cleanUsername, password }).catch(() => {});
      } else {
        await invoke("clear_offline_profile").catch(() => {});
      }
      onLogin(account);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleMicrosoftLogin = async () => {
    setLoading(true);
    setError("");
    try {
      const account = await invoke<Account>("login_microsoft");
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
          <h1 className="auth-modal-title">Добро пожаловать</h1>
          <p className="auth-modal-subtitle">Выберите способ входа в лаунчер</p>
        </motion.div>

        
        <motion.div className="auth-modal-methods" custom={1} variants={itemVariants} initial="hidden" animate="visible">
          
          <motion.button
            className={`auth-method-card${selectedMethod === "offline" ? " selected" : ""}`}
            onClick={() => { setSelectedMethod("offline"); setError(""); setPassword(""); }}
            whileHover={{ scale: 1.015 }}
            whileTap={{ scale: 0.99 }}
          >
            <div className="auth-method-icon offline-icon">
              <OfflineWifiOffIcon />
            </div>
            <div className="auth-method-info">
              <span className="auth-method-name">Офлайн режим</span>
              <span className="auth-method-desc">Без пароля, нельзя использовать занятые RPWorld ники</span>
            </div>
            <div className="auth-method-arrow">
              <ArrowIcon />
            </div>
          </motion.button>

          
          <motion.button
            className={`auth-method-card${selectedMethod === "rpworld" ? " selected" : ""}`}
            onClick={() => { setSelectedMethod("rpworld"); setError(""); }}
            whileHover={{ scale: 1.015 }}
            whileTap={{ scale: 0.99 }}
          >
            <div className="auth-method-icon rpworld-icon">
              <RpworldAccountIcon />
            </div>
            <div className="auth-method-info">
              <span className="auth-method-name">RPWorld аккаунт</span>
              <span className="auth-method-desc">Вход по нику и паролю из системы RPWorld</span>
            </div>
            <div className="auth-method-arrow">
              <ArrowIcon />
            </div>
          </motion.button>

          
          <motion.button
            className={`auth-method-card${selectedMethod === "microsoft" ? " selected" : ""}`}
            onClick={() => { setSelectedMethod("microsoft"); setError(""); }}
            whileHover={{ scale: 1.015 }}
            whileTap={{ scale: 0.99 }}
          >
            <div className="auth-method-icon microsoft-icon">
              <MicrosoftIcon />
            </div>
            <div className="auth-method-info">
              <span className="auth-method-name">Microsoft</span>
              <span className="auth-method-desc">Лицензионный аккаунт</span>
            </div>
            <div className="auth-method-badge">
              <ShieldIcon />
            </div>
            <div className="auth-method-arrow">
              <ArrowIcon />
            </div>
          </motion.button>
        </motion.div>

        
        <AnimatePresence mode="wait">
          {(selectedMethod === "offline" || selectedMethod === "rpworld") && (
            <motion.div
              key={`${selectedMethod}-form`}
              className="auth-modal-form"
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
            >
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
                    onKeyDown={(e) => e.key === "Enter" && (selectedMethod === "rpworld" ? handleRpworldLogin() : handleOfflineLogin())}
                    maxLength={16}
                    autoFocus
                  />
                </div>
                {selectedMethod === "rpworld" && (
                  <>
                    <div className="auth-input-wrap" style={{ marginTop: 10 }}>
                      <div className="auth-input-icon">
                        <ShieldIcon />
                      </div>
                      <input
                        type="password"
                        className="auth-modal-input"
                        placeholder="Пароль RPWorld аккаунта..."
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleRpworldLogin()}
                      />
                    </div>

                    <label className="auth-save-profile">
                      <input type="checkbox" checked={saveProfile} onChange={(e) => setSaveProfile(e.target.checked)} />
                      <span>Сохранить профиль для автозаполнения</span>
                    </label>
                  </>
                )}

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
                  onClick={selectedMethod === "rpworld" ? handleRpworldLogin : handleOfflineLogin}
                  disabled={loading || !username.trim() || (selectedMethod === "rpworld" && !password.trim())}
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                >
                  {loading ? (
                    <span className="auth-modal-loading">
                      <div className="spinner" style={{ width: 16, height: 16, borderWidth: 2 }} />
                      Вход...
                    </span>
                  ) : (
                    "Войти"
                  )}
                </motion.button>
              </div>
            </motion.div>
          )}

          {selectedMethod === "microsoft" && (
            <motion.div
              key="microsoft-form"
              className="auth-modal-form"
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
            >
              <div className="auth-form-inner">
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
                  className="auth-modal-submit microsoft"
                  onClick={handleMicrosoftLogin}
                  disabled={loading}
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                >
                  {loading ? (
                    <span className="auth-modal-loading">
                      <div className="spinner" style={{ width: 16, height: 16, borderWidth: 2 }} />
                      Авторизация...
                    </span>
                  ) : (
                    <span className="auth-modal-loading">
                      <MicrosoftIcon />
                      Войти через Microsoft
                    </span>
                  )}
                </motion.button>

                <p className="auth-modal-hint">
                  Откроется браузер для авторизации через ваш аккаунт Microsoft
                </p>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        
        <motion.div className="auth-modal-footer" custom={3} variants={itemVariants} initial="hidden" animate="visible">
          <div className="auth-modal-divider" />
          <span className="auth-modal-version">RPWorld Launcher</span>
        </motion.div>
      </motion.div>
    </motion.div>
  );
}