import { useState } from "react";
import { useAuth } from "../hooks/useAuth";
import { discordLoginUrl } from "../api/client";
import { AdminAnnouncementsTab } from "./admin/AdminAnnouncementsTab";
import { AdminContentTab } from "./admin/AdminContentTab";
import { AdminEventsTab } from "./admin/AdminEventsTab";
import { AdminRegistrationsTab } from "./admin/AdminRegistrationsTab";
import "./AdminPage.css";

const TABS = [
  { id: "content", label: "Content" },
  { id: "events", label: "Events" },
  { id: "announcements", label: "Announcements" },
  { id: "registrations", label: "Registrations" },
] as const;

type TabId = (typeof TABS)[number]["id"];

export function AdminPage() {
  const { isAdmin, isLoading: authLoading, isAuthenticated } = useAuth();
  const [tab, setTab] = useState<TabId>("content");

  if (authLoading) return <div className="loading">Checking access…</div>;

  if (!isAuthenticated) {
    return (
      <div className="admin-gate">
        <h1>Admin</h1>
        <p>Sign in with Discord to access the admin panel.</p>
        <a className="btn btn-primary" href={discordLoginUrl("/admin")}>
          Login with Discord
        </a>
      </div>
    );
  }

  if (!isAdmin) {
    return (
      <div className="admin-gate">
        <h1>Access denied</h1>
        <p>Your Discord account does not have admin permissions.</p>
      </div>
    );
  }

  return (
    <div>
      <div className="admin-tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`admin-tab ${tab === t.id ? "active" : ""}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>
      {tab === "content" && <AdminContentTab />}
      {tab === "events" && <AdminEventsTab />}
      {tab === "announcements" && <AdminAnnouncementsTab />}
      {tab === "registrations" && <AdminRegistrationsTab />}
    </div>
  );
}
