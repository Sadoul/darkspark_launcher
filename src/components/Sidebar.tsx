import { useEffect, useMemo, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import type { CustomModpack } from "../App";

const NAV_ORDER_STORAGE_KEY = "darkspark_nav_order";

export type Page = "darkspark" | "minigames" | "custom" | "settings" | "admin" | `custom:${string}`;

interface SidebarProps {
  currentPage: Page;
  onPageChange: (page: Page) => void;
  account: { username: string; account_type: string; is_admin?: boolean } | null;

  onLogout: () => void;
  avatarUrl: string | null;
  customModpacks: CustomModpack[];
  onConfigurePage: (page: Page) => void;
  onDeleteBuiltinModpack: (page: Page) => void;
  onDeleteCustomModpack: (name: string) => void;
}


const IconGlobe = () => (
  <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg" width="16" height="16">
    <circle cx="10" cy="10" r="7.5" stroke="currentColor" strokeWidth="1.4"/>
    <path d="M10 2.5C10 2.5 7 5.5 7 10s3 7.5 3 7.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
    <path d="M10 2.5C10 2.5 13 5.5 13 10s-3 7.5-3 7.5" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
    <path d="M2.5 10h15" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
    <path d="M3.5 6.5h13M3.5 13.5h13" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round"/>
  </svg>
);

const IconZap = () => (
  <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg" width="16" height="16">
    <path d="M11.5 2L4 11h7l-2.5 7L18 9h-7l.5-7z" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round"/>
  </svg>
);

const IconBox = () => (
  <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg" width="16" height="16">
    <path d="M10 2L2.5 6v8L10 18l7.5-4V6L10 2z" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round"/>
    <path d="M10 2v16M2.5 6l7.5 4 7.5-4" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
  </svg>
);

const IconSettings = () => (
  <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg" width="16" height="16">
    <circle cx="10" cy="10" r="2.5" stroke="currentColor" strokeWidth="1.4"/>
    <path d="M10 1.5v2M10 16.5v2M1.5 10h2M16.5 10h2M3.7 3.7l1.42 1.42M14.88 14.88l1.42 1.42M3.7 16.3l1.42-1.42M14.88 5.12l1.42-1.42" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/>
  </svg>
);

const IconLock = () => (
  <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="11" height="11">
    <rect x="3" y="7" width="10" height="7" rx="1.5" stroke="currentColor" strokeWidth="1.3"/>
    <path d="M5.5 7V5a2.5 2.5 0 015 0v2" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
  </svg>
);

const IconEject = () => (
  <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" width="18" height="18">
    <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
    <polyline points="16 17 21 12 16 7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
    <line x1="21" y1="12" x2="9" y2="12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);

type NavItem = { id: Page; label: string; icon: React.ReactElement; locked?: boolean; customName?: string }; 
type ContextMenuState = { x: number; y: number; item: NavItem } | null;

const NAV_ITEMS: NavItem[] = [
  { id: "darkspark",   label: "DarkSpark",      icon: <IconGlobe /> },
  { id: "minigames", label: "Мини-игры",    icon: <IconZap />, locked: true },
  { id: "custom",    label: "Свой модпак",  icon: <IconBox /> },
  { id: "settings",  label: "Настройки",    icon: <IconSettings /> },
];

const DISCORD_URL = "https://discord.gg/DnVNeBYzMM";

export default function Sidebar({ currentPage, onPageChange, account, onLogout, avatarUrl, customModpacks, onConfigurePage, onDeleteBuiltinModpack, onDeleteCustomModpack }: SidebarProps) {
  const [contextMenu, setContextMenu] = useState<ContextMenuState>(null);
  const [navOrder, setNavOrder] = useState<string[]>(() => {
    try {
      const raw = localStorage.getItem(NAV_ORDER_STORAGE_KEY);
      return raw ? (JSON.parse(raw) as string[]) : [];
    } catch {
      return [];
    }
  });
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);

  const baseNavItems: NavItem[] = useMemo(
    () => [
      ...NAV_ITEMS,
      ...(account?.is_admin
        ? [{ id: "admin" as Page, label: "Админ", icon: <IconSettings /> }]
        : []),

      ...customModpacks.map((pack) => ({
        id: `custom:${pack.name}` as Page,
        label: pack.name,
        icon: <IconBox />,
        customName: pack.name,
      })),
    ],
    [account?.is_admin, customModpacks],

  );

  const navItems: NavItem[] = useMemo(() => {
    if (navOrder.length === 0) return baseNavItems;
    const byId = new Map(baseNavItems.map((item) => [item.id, item]));
    const ordered: NavItem[] = [];
    for (const id of navOrder) {
      const found = byId.get(id as Page);
      if (found) {
        ordered.push(found);
        byId.delete(id as Page);
      }
    }
    return [...ordered, ...byId.values()];
  }, [baseNavItems, navOrder]);

  useEffect(() => {
    if (navOrder.length === 0) return;
    const valid = new Set(baseNavItems.map((item) => item.id as string));
    const filtered = navOrder.filter((id) => valid.has(id));
    if (filtered.length !== navOrder.length) {
      setNavOrder(filtered);
      try { localStorage.setItem(NAV_ORDER_STORAGE_KEY, JSON.stringify(filtered)); } catch (e) {
        // error ignored
      }
    }
  }, [baseNavItems, navOrder]);

  const persistOrder = (items: NavItem[]) => {
    const order = items.map((item) => item.id as string);
    setNavOrder(order);
    try { localStorage.setItem(NAV_ORDER_STORAGE_KEY, JSON.stringify(order)); } catch (e) {
        // error ignored
      }
  };


  const dragStateRef = useRef<{
    id: string;
    startX: number;
    startY: number;
    started: boolean;
    pointerId: number;
  } | null>(null);
  const navRef = useRef<HTMLElement | null>(null);
  const suppressClickRef = useRef(false);

  const reorderToIndex = (sourceId: string, insertionIndex: number) => {
    if (!sourceId) return;
    const sourceIndex = navItems.findIndex((item) => item.id === sourceId);
    if (sourceIndex === -1) return;

    const next = navItems.slice();
    const [moved] = next.splice(sourceIndex, 1);
    const safeIndex = Math.max(0, Math.min(insertionIndex, next.length));
    next.splice(safeIndex, 0, moved);

    const currentOrder = navItems.map((item) => item.id).join("|");
    const nextOrder = next.map((item) => item.id).join("|");
    if (currentOrder === nextOrder) return;
    persistOrder(next);
  };

  const getInsertionIndexByY = (draggedId: string, y: number): number | null => {
    if (!navRef.current) return null;
    const items = Array.from(navRef.current.children) as HTMLElement[];
    const visibleItems = items.filter((el) => el.dataset.navId && el.dataset.navId !== draggedId);
    if (visibleItems.length === 0) return null;

    for (let index = 0; index < visibleItems.length; index += 1) {
      const rect = visibleItems[index].getBoundingClientRect();
      const insertionLine = rect.top + rect.height * 0.72;
      if (y < insertionLine) return index;
    }
    return visibleItems.length;
  };

  const findNavItemIdAt = (x: number, y: number): string | null => {
    if (!navRef.current) return null;
    for (const child of Array.from(navRef.current.children)) {
      const el = child as HTMLElement;
      const rect = el.getBoundingClientRect();
      if (y >= rect.top && y <= rect.bottom && x >= rect.left && x <= rect.right) {
        return el.dataset.navId ?? null;
      }
    }
    return null;
  };

  const handlePointerDown = (event: React.PointerEvent<HTMLButtonElement>, id: string) => {
    if (event.button !== 0 || event.pointerType === "touch") return;
    dragStateRef.current = {
      id,
      startX: event.clientX,
      startY: event.clientY,
      started: false,
      pointerId: event.pointerId,
    };
  };

  const handlePointerMove = (event: React.PointerEvent<HTMLButtonElement>) => {
    const state = dragStateRef.current;
    if (!state || state.pointerId !== event.pointerId) return;
    if ((event.buttons & 1) === 0) {
      finishDrag(event, false);
      return;
    }
    if (!state.started) {
      const dx = event.clientX - state.startX;
      const dy = event.clientY - state.startY;
      if (dx * dx + dy * dy < 25) return;
      state.started = true;
      setDragId(state.id);
      try { (event.currentTarget as HTMLButtonElement).setPointerCapture(event.pointerId); } catch (e) {
        // error ignored
      }
    }
    const hoverId = findNavItemIdAt(event.clientX, event.clientY);
    setOverId(hoverId);
    const insertionIndex = getInsertionIndexByY(state.id, event.clientY);
    if (insertionIndex !== null) {
      reorderToIndex(state.id, insertionIndex);
    }
  };

  const finishDrag = (event: React.PointerEvent<HTMLButtonElement>, commit: boolean) => {
    const state = dragStateRef.current;
    dragStateRef.current = null;
    if (!state) return;
    try { (event.currentTarget as HTMLButtonElement).releasePointerCapture(event.pointerId); } catch (e) {
        // error ignored
      }
    if (state.started && commit) {

    }
    if (state.started) suppressClickRef.current = true;
    setDragId(null);
    setOverId(null);
    window.setTimeout(() => { suppressClickRef.current = false; }, 120);
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLButtonElement>) => finishDrag(event, true);
  const handlePointerCancel = (event: React.PointerEvent<HTMLButtonElement>) => finishDrag(event, false);

  const openContextMenu = (event: React.MouseEvent, item: NavItem) => {
    event.preventDefault();
    event.stopPropagation();
    if (item.id === "settings" || item.id === "custom" || item.id === "admin") return;
    setContextMenu({ x: event.clientX, y: event.clientY, item });
  };
  const handleDiscord = async () => {
    try { await invoke("open_url", { url: DISCORD_URL }); } catch { window.open(DISCORD_URL, "_blank"); }
  };

  const handleOpenFolder = async () => {
    try { await invoke("open_data_folder"); } catch (e) {
        // error ignored
      }
  };

  return (
    <aside className="sidebar">
      
      <div className="sidebar-logo">
        <img src="/icons/Inside.png" alt="DarkSpark" className="sidebar-logo-img" draggable={false} />
        <div className="sidebar-logo-text">
          <div className="sidebar-logo-name">DarkSpark</div>
          <div className="sidebar-logo-sub">Launcher</div>
        </div>
      </div>

      
      <nav className="sidebar-nav" ref={navRef as React.Ref<HTMLElement>}>
        {navItems.map((item) => (
          <motion.button
            key={item.id}
            data-nav-id={item.id}
            className={`nav-item${currentPage === item.id ? " active" : ""}${item.locked ? " locked" : ""}${dragId === item.id ? " dragging" : ""}${overId === item.id && dragId && dragId !== item.id ? " drag-over" : ""}`}
            onClick={() => {
              if (dragId || suppressClickRef.current) return;
              if (!item.locked) onPageChange(item.id);
            }}
            onContextMenu={(event) => openContextMenu(event, item)}
            whileHover={item.locked || dragId ? {} : { x: 2 }}
            whileTap={item.locked || dragId ? {} : { scale: 0.97 }}
            layout
            onPointerDown={(event) => handlePointerDown(event as unknown as React.PointerEvent<HTMLButtonElement>, item.id as string)}
            onPointerMove={(event) => handlePointerMove(event as unknown as React.PointerEvent<HTMLButtonElement>)}
            onPointerUp={(event) => handlePointerUp(event as unknown as React.PointerEvent<HTMLButtonElement>)}
            onPointerCancel={(event) => handlePointerCancel(event as unknown as React.PointerEvent<HTMLButtonElement>)}
            style={{ touchAction: "none" }}
          >
            <span className="nav-icon">{item.icon}</span>
            <span className="nav-label">{item.label}</span>
            {item.locked && <span className="nav-lock"><IconLock /></span>}
            <AnimatePresence>
              {currentPage === item.id && (
                <motion.span
                  className="nav-active-bar"
                  layoutId="active-bar"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.2 }}
                />
              )}
            </AnimatePresence>
          </motion.button>
        ))}
      </nav>

      <AnimatePresence>
        {contextMenu && (
          <motion.div
            className="sidebar-context-menu"
            style={{ left: contextMenu.x, top: contextMenu.y }}
            initial={{ opacity: 0, scale: 0.96 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.96 }}
            onMouseLeave={() => setContextMenu(null)}
          >
            <button
              className="sidebar-context-item"
              onClick={() => {
                onConfigurePage(contextMenu.item.id);
                setContextMenu(null);
              }}
              disabled={contextMenu.item.id === "minigames"}
            >
              Настроить {contextMenu.item.id === "minigames" && <span className="context-lock"><IconLock /></span>}
            </button>
            {(contextMenu.item.id === "rpworld" || contextMenu.item.id === "minigames") && (
              <button
                className="sidebar-context-item danger"
                disabled={contextMenu.item.id === "minigames"}
                onClick={() => {
                  onDeleteBuiltinModpack(contextMenu.item.id);
                  setContextMenu(null);
                }}
              >
                Удалить сборку {contextMenu.item.id === "minigames" && <span className="context-lock"><IconLock /></span>}
              </button>
            )}
            {contextMenu.item.customName && (
              <button
                className="sidebar-context-item danger"
                onClick={() => {
                  onDeleteCustomModpack(contextMenu.item.customName!);
                  setContextMenu(null);
                }}
              >
                Удалить
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>

      <div className="sidebar-spacer" />

      
      <div className="sidebar-actions-row">
        <motion.button
          className="sidebar-icon-btn"
          onClick={handleOpenFolder}
          title="Открыть папку данных"
          whileHover={{ scale: 1.08 }}
          whileTap={{ scale: 0.93 }}
        >
          <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M2 5.5A1.5 1.5 0 013.5 4h3.382a1.5 1.5 0 011.06.44l.94.94H16.5A1.5 1.5 0 0118 7v8a1.5 1.5 0 01-1.5 1.5h-13A1.5 1.5 0 012 15V5.5z"
              stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />
          </svg>
        </motion.button>

        <motion.button
          className="sidebar-icon-btn"
          onClick={handleDiscord}
          title="Discord сервер RPWorld"
          whileHover={{ scale: 1.08 }}
          whileTap={{ scale: 0.93 }}
          style={{ flex: 1 }}
        >
          <img src="/icons/discord.png" alt="Discord" style={{ width: 18, height: 18, objectFit: "contain" }} />
          <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-2)", marginLeft: 6 }}>Discord</span>
        </motion.button>
      </div>

      
      {account && (
        <div className="sidebar-account">
          <div className="account-avatar">
            {avatarUrl ? (
              <img src={avatarUrl} alt="avatar" style={{ width: "100%", height: "100%", objectFit: "cover", borderRadius: "50%" }} />
            ) : (
              account.username[0]?.toUpperCase()
            )}
          </div>
          <div className="account-info">
            <div className="account-name">{account.username}</div>
            <div className="account-type">{account.account_type === "offline" ? "Офлайн" : "Microsoft"}</div>
          </div>
          <motion.button
            className="logout-btn"
            onClick={onLogout}
            whileHover={{ scale: 1.12 }}
            whileTap={{ scale: 0.9 }}
            title="Выйти"
          >
            <IconEject />
          </motion.button>
        </div>
      )}
    </aside>
  );
}