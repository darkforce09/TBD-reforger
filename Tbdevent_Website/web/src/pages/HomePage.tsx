import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { api } from "../api/client";
import { Countdown } from "../components/Countdown";
import { MarkdownRenderer } from "../components/MarkdownRenderer";
import "./HomePage.css";

export function HomePage() {
  const nextEvent = useQuery({ queryKey: ["events", "next"], queryFn: () => api.getNextEvent() });
  const announcements = useQuery({
    queryKey: ["announcements", { limit: 3 }],
    queryFn: () => api.listAnnouncements(3),
  });

  const event = nextEvent.data;

  return (
    <div className="dashboard">
      <header className="page-header">
        <h1 className="page-title">TBD Event Hub</h1>
        <p className="page-subtitle">
          Your central place for TBD PvP event information and sign-ups.
        </p>
      </header>

      <section className="dashboard-grid">
        <div className="dashboard-card card dashboard-card--featured">
          <h2>Next Event</h2>
          {nextEvent.isLoading && <p className="text-muted">Loading…</p>}
          {event ? (
            <>
              <h3 className="dashboard-event-title">
                <Link to={`/events/${event.slug}`}>{event.title}</Link>
              </h3>
              <p className="dashboard-event-meta">
                {new Date(event.startsAt).toLocaleString()}
                {event.mapName && ` · ${event.mapName}`}
              </p>
              <p className="dashboard-countdown">
                Starts in <Countdown target={event.startsAt} />
              </p>
              <p className="dashboard-slots">
                {event.maxPlayers != null
                  ? `${event.registeredCount}/${event.maxPlayers} slots filled`
                  : `${event.registeredCount} registered`}
              </p>
              {event.signupsOpen && (
                <Link to={`/events/${event.slug}`} className="btn btn-primary">
                  Sign up now
                </Link>
              )}
            </>
          ) : (
            !nextEvent.isLoading && (
              <p className="text-muted">No upcoming events scheduled.</p>
            )
          )}
        </div>

        <div className="dashboard-card card">
          <h2>Quick Links</h2>
          <ul className="quick-links">
            <li><Link to="/events">All Events</Link></li>
            <li><Link to="/rules">Rules</Link></li>
            <li><Link to="/server">Server Info</Link></li>
            <li><Link to="/announcements">Announcements</Link></li>
          </ul>
        </div>
      </section>

      <section className="dashboard-announcements">
        <div className="section-header">
          <h2>Latest Announcements</h2>
          <Link to="/announcements">View all</Link>
        </div>
        {announcements.isLoading && <p className="text-muted">Loading…</p>}
        {announcements.data?.map((a) => (
          <article key={a.id} className="card announcement-preview">
            {a.pinned && <span className="badge badge--pinned">Pinned</span>}
            <h3>{a.title}</h3>
            <MarkdownRenderer content={a.body} />
          </article>
        ))}
        {announcements.data?.length === 0 && (
          <p className="text-muted">No announcements yet.</p>
        )}
      </section>
    </div>
  );
}
