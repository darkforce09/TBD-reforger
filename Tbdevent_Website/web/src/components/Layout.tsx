import { useState } from "react";
import { Link, Outlet } from "react-router-dom";
import { discordLoginUrl } from "../api/client";
import { useAuth } from "../hooks/useAuth";
import { Sidebar } from "./Sidebar";
import "./Layout.css";

export function Layout() {
  const { isAdmin, isAuthenticated, user, logout } = useAuth();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  return (
    <div className="layout">
      <header className="top-bar">
        <button
          className="menu-toggle btn"
          onClick={() => setSidebarOpen((o) => !o)}
          aria-label="Toggle menu"
        >
          ☰
        </button>
        <Link to="/" className="brand">
          <span className="brand-mark">TBD</span>
          <span className="brand-text">Event — Arma Reforger</span>
        </Link>
        <div className="auth-area">
          {isAuthenticated && user ? (
            <div className="user-chip">
              {user.avatarUrl && (
                <img src={user.avatarUrl} alt="" className="avatar" />
              )}
              <span>{user.username}</span>
              <button className="btn btn-small" onClick={() => logout()}>
                Logout
              </button>
            </div>
          ) : (
              <a className="btn btn-small" href={discordLoginUrl()}>
                Login with Discord
              </a>
          )}
        </div>
      </header>
      <div className="layout-body">
        <Sidebar
          isAuthenticated={isAuthenticated}
          isAdmin={isAdmin}
          open={sidebarOpen}
          onClose={() => setSidebarOpen(false)}
        />
        <main className="site-main">
          <div className="content-container">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
