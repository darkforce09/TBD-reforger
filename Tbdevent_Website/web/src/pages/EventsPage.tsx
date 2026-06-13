import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { EventCard } from "../components/EventCard";

export function EventsPage() {
  const [tab, setTab] = useState<"upcoming" | "past">("upcoming");

  const { data, isLoading } = useQuery({
    queryKey: ["events", tab],
    queryFn: () =>
      api.listEvents(tab === "upcoming" ? { upcoming: true } : { past: true }),
  });

  return (
    <div>
      <header className="page-header">
        <h1 className="page-title">Events</h1>
        <p className="page-subtitle">Upcoming and past TBD PvP events.</p>
      </header>

      <div className="tab-bar">
        <button
          className={`btn ${tab === "upcoming" ? "btn-primary" : ""}`}
          onClick={() => setTab("upcoming")}
        >
          Upcoming
        </button>
        <button
          className={`btn ${tab === "past" ? "btn-primary" : ""}`}
          onClick={() => setTab("past")}
        >
          Past
        </button>
      </div>

      {isLoading && <p className="text-muted">Loading…</p>}
      <div className="events-list">
        {data?.map((e) => <EventCard key={e.id} event={e} />)}
      </div>
      {data?.length === 0 && !isLoading && (
        <p className="text-muted">No {tab} events.</p>
      )}

      <style>{`
        .tab-bar { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; }
        .events-list { display: flex; flex-direction: column; gap: 0.75rem; }
      `}</style>
    </div>
  );
}
