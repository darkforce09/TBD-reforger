import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router-dom";
import { api } from "../api/client";
import { Countdown } from "../components/Countdown";
import { RegistrationRoster } from "../components/RegistrationRoster";
import { SignupButton } from "../components/SignupButton";
import { MarkdownRenderer } from "../components/MarkdownRenderer";

export function EventDetailPage() {
  const { slug } = useParams<{ slug: string }>();

  const eventQuery = useQuery({
    queryKey: ["event", slug],
    queryFn: () => api.getEvent(slug!),
    enabled: !!slug,
  });

  const rosterQuery = useQuery({
    queryKey: ["roster", slug],
    queryFn: () => api.getRoster(slug!),
    enabled: !!slug,
  });

  if (eventQuery.isLoading) return <div className="loading">Loading…</div>;
  if (eventQuery.error || !eventQuery.data) {
    return <div className="error">Event not found.</div>;
  }

  const event = eventQuery.data;

  return (
    <div>
      <header className="page-header">
        <h1 className="page-title">{event.title}</h1>
        <p className="page-subtitle">
          {new Date(event.startsAt).toLocaleString()}
          {event.mapName && ` · ${event.mapName}`}
        </p>
      </header>

      <div className="info-box" style={{ marginBottom: "1.5rem" }}>
        <p>
          Starts in <Countdown target={event.startsAt} />
          {" · "}
          {event.maxPlayers != null
            ? `${event.registeredCount}/${event.maxPlayers} slots filled`
            : `${event.registeredCount} registered`}
          {(event.waitlistCount ?? 0) > 0 &&
            ` · ${event.waitlistCount} on waitlist`}
        </p>
        <SignupButton event={event} />
      </div>

      {event.description && (
        <section className="page-section">
          <h2 className="section-title">About</h2>
          <div className="section-content">
            <MarkdownRenderer content={event.description} />
          </div>
        </section>
      )}

      <section className="page-section">
        <h2 className="section-title">Registered Players</h2>
        <div className="section-content">
          {rosterQuery.isLoading ? (
            <p className="text-muted">Loading roster…</p>
          ) : (
            <RegistrationRoster roster={rosterQuery.data ?? []} />
          )}
        </div>
      </section>
    </div>
  );
}
