import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type CreateEventInput, type Event } from "../../api/client";

const emptyForm = (): CreateEventInput => ({
  title: "",
  slug: "",
  description: "",
  mapName: "",
  startsAt: new Date(Date.now() + 86400000 * 7).toISOString().slice(0, 16),
  status: "draft",
  signupsOpen: false,
  published: false,
});

export function AdminEventsTab() {
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState<Event | null>(null);
  const [form, setForm] = useState<CreateEventInput>(emptyForm());
  const [message, setMessage] = useState<string | null>(null);

  const { data: events, isLoading } = useQuery({
    queryKey: ["admin", "events"],
    queryFn: () => api.adminListEvents(),
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const payload = {
        ...form,
        startsAt: new Date(form.startsAt).toISOString(),
        endsAt: form.endsAt ? new Date(form.endsAt).toISOString() : undefined,
      };
      if (editing) {
        return api.adminUpdateEvent(editing.id, payload);
      }
      return api.adminCreateEvent(payload);
    },
    onSuccess: () => {
      setMessage("Event saved.");
      setEditing(null);
      setForm(emptyForm());
      queryClient.invalidateQueries({ queryKey: ["admin", "events"] });
      queryClient.invalidateQueries({ queryKey: ["events"] });
    },
    onError: (err: Error) => setMessage(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.adminDeleteEvent(id),
    onSuccess: () => {
      setMessage("Event deleted.");
      queryClient.invalidateQueries({ queryKey: ["admin", "events"] });
    },
    onError: (err: Error) => setMessage(err.message),
  });

  const startEdit = (event: Event) => {
    setEditing(event);
    setForm({
      title: event.title,
      slug: event.slug,
      description: event.description,
      mapName: event.mapName,
      startsAt: event.startsAt.slice(0, 16),
      endsAt: event.endsAt?.slice(0, 16),
      status: event.status,
      maxPlayers: event.maxPlayers,
      signupsOpen: event.signupsOpen,
      published: event.published,
    });
  };

  return (
    <div>
      <h1>Manage Events</h1>
      <div className="admin-form card">
        <h2>{editing ? "Edit Event" : "New Event"}</h2>
        <div className="admin-form-grid">
          <label>Title<input className="field" value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} /></label>
          <label>Slug<input className="field" value={form.slug} onChange={(e) => setForm({ ...form, slug: e.target.value })} /></label>
          <label>Map<input className="field" value={form.mapName} onChange={(e) => setForm({ ...form, mapName: e.target.value })} /></label>
          <label>Max players<input className="field" type="number" value={form.maxPlayers ?? ""} onChange={(e) => setForm({ ...form, maxPlayers: e.target.value ? Number(e.target.value) : undefined })} /></label>
          <label>Starts at<input className="field" type="datetime-local" value={form.startsAt} onChange={(e) => setForm({ ...form, startsAt: e.target.value })} /></label>
          <label>Ends at<input className="field" type="datetime-local" value={form.endsAt ?? ""} onChange={(e) => setForm({ ...form, endsAt: e.target.value })} /></label>
          <label>Status
            <select className="field" value={form.status} onChange={(e) => setForm({ ...form, status: e.target.value })}>
              <option value="draft">Draft</option>
              <option value="published">Published</option>
              <option value="live">Live</option>
              <option value="completed">Completed</option>
              <option value="cancelled">Cancelled</option>
            </select>
          </label>
          <label className="published-toggle"><input type="checkbox" checked={form.signupsOpen} onChange={(e) => setForm({ ...form, signupsOpen: e.target.checked })} /> Sign-ups open</label>
          <label className="published-toggle"><input type="checkbox" checked={form.published} onChange={(e) => setForm({ ...form, published: e.target.checked })} /> Published</label>
        </div>
        <label>Description<textarea className="field section-textarea" rows={4} value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} /></label>
        <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.75rem" }}>
          <button className="btn btn-primary" disabled={saveMutation.isPending} onClick={() => saveMutation.mutate()}>
            {saveMutation.isPending ? "Saving…" : "Save event"}
          </button>
          {editing && <button className="btn" onClick={() => { setEditing(null); setForm(emptyForm()); }}>Cancel</button>}
        </div>
      </div>

      {isLoading && <p className="text-muted">Loading…</p>}
      <ul className="admin-list">
        {events?.map((e) => (
          <li key={e.id} className="card admin-list-item">
            <div>
              <strong>{e.title}</strong>
              <span className="text-muted"> · {e.slug} · {e.status}</span>
            </div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button className="btn btn-small" onClick={() => startEdit(e)}>Edit</button>
              <button className="btn btn-danger btn-small" onClick={() => deleteMutation.mutate(e.id)}>Delete</button>
            </div>
          </li>
        ))}
      </ul>
      {message && <p className="admin-message">{message}</p>}
    </div>
  );
}
