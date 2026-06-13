import { Route, Routes } from "react-router-dom";
import { Layout } from "./components/Layout";
import { AdminPage } from "./pages/AdminPage";
import { AnnouncementsPage } from "./pages/AnnouncementsPage";
import { CompliancePage } from "./pages/CompliancePage";
import { EventDetailPage } from "./pages/EventDetailPage";
import { EventsPage } from "./pages/EventsPage";
import { HomePage } from "./pages/HomePage";
import { ModsPage } from "./pages/ModsPage";
import { MyEventsPage } from "./pages/MyEventsPage";
import { RulesPage } from "./pages/RulesPage";
import { ServerPage } from "./pages/ServerPage";

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<HomePage />} />
        <Route path="announcements" element={<AnnouncementsPage />} />
        <Route path="events" element={<EventsPage />} />
        <Route path="events/:slug" element={<EventDetailPage />} />
        <Route path="my-events" element={<MyEventsPage />} />
        <Route path="rules" element={<RulesPage />} />
        <Route path="compliance" element={<CompliancePage />} />
        <Route path="server" element={<ServerPage />} />
        <Route path="mods" element={<ModsPage />} />
        <Route path="admin" element={<AdminPage />} />
      </Route>
    </Routes>
  );
}
