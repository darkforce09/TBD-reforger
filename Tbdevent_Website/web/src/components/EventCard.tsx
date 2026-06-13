import { Link } from "react-router-dom";
import type { EventSummary } from "../api/client";

type Props = { event: EventSummary };

function formatDate(iso: string) {
  return new Date(iso).toLocaleString(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function slotsLabel(event: EventSummary) {
  if (event.maxPlayers == null) return `${event.registeredCount} registered`;
  return `${event.registeredCount}/${event.maxPlayers} slots`;
}

export function EventCard({ event }: Props) {
  return (
    <Link to={`/events/${event.slug}`} className="event-card card">
      <div className="event-card-header">
        <h2>{event.title}</h2>
        {event.signupsOpen && <span className="badge badge--open">Sign-ups open</span>}
      </div>
      <p className="event-card-meta">
        {formatDate(event.startsAt)}
        {event.mapName && ` · ${event.mapName}`}
      </p>
      <p className="event-card-slots">{slotsLabel(event)}</p>
    </Link>
  );
}
