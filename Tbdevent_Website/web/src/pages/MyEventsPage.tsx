import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { api } from "../api/client";
import { useAuth } from "../hooks/useAuth";
import { discordLoginUrl } from "../api/client";

export function MyEventsPage() {
  const { isAuthenticated, isLoading: authLoading } = useAuth();
  const queryClient = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ["my-registrations"],
    queryFn: () => api.myRegistrations(),
    enabled: isAuthenticated,
  });

  const cancelMutation = useMutation({
    mutationFn: (slug: string) => api.cancelRegistration(slug),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["my-registrations"] }),
  });

  if (authLoading) return <div className="loading">Loading…</div>;

  if (!isAuthenticated) {
    return (
      <div className="admin-gate">
        <h1>My Events</h1>
        <p>Sign in with Discord to see your event registrations.</p>
        <a className="btn btn-primary" href={discordLoginUrl("/my-events")}>
          Login with Discord
        </a>
      </div>
    );
  }

  return (
    <div>
      <header className="page-header">
        <h1 className="page-title">My Events</h1>
        <p className="page-subtitle">Your upcoming event registrations.</p>
      </header>

      {isLoading && <p className="text-muted">Loading…</p>}
      {data?.map((reg) => (
        <article key={reg.id} className="card" style={{ marginBottom: "0.75rem" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
            <div>
              <h2 style={{ margin: 0, fontSize: "1.1rem" }}>
                <Link to={`/events/${reg.event?.slug}`}>{reg.event?.title}</Link>
              </h2>
              <p className="text-muted" style={{ margin: "0.25rem 0", fontSize: "0.85rem" }}>
                {reg.event && new Date(reg.event.startsAt).toLocaleString()}
              </p>
              <span className={`badge badge--${reg.status === "waitlist" ? "waitlist" : "registered"}`}>
                {reg.status}
              </span>
            </div>
            <button
              className="btn btn-danger btn-small"
              disabled={cancelMutation.isPending}
              onClick={() => reg.event && cancelMutation.mutate(reg.event.slug)}
            >
              Cancel
            </button>
          </div>
        </article>
      ))}
      {data?.length === 0 && !isLoading && (
        <p className="text-muted">
          No upcoming registrations. <Link to="/events">Browse events</Link>
        </p>
      )}
    </div>
  );
}
