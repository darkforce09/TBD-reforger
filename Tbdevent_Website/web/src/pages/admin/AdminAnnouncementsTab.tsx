import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type Announcement, type CreateAnnouncementInput } from "../../api/client";

const emptyForm = (): CreateAnnouncementInput => ({
  title: "",
  body: "",
  pinned: false,
  published: false,
});

export function AdminAnnouncementsTab() {
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState<Announcement | null>(null);
  const [form, setForm] = useState<CreateAnnouncementInput>(emptyForm());
  const [message, setMessage] = useState<string | null>(null);

  const { data: items, isLoading } = useQuery({
    queryKey: ["admin", "announcements"],
    queryFn: () => api.adminListAnnouncements(),
  });

  const saveMutation = useMutation({
    mutationFn: () =>
      editing
        ? api.adminUpdateAnnouncement(editing.id, form)
        : api.adminCreateAnnouncement(form),
    onSuccess: () => {
      setMessage("Announcement saved.");
      setEditing(null);
      setForm(emptyForm());
      queryClient.invalidateQueries({ queryKey: ["admin", "announcements"] });
      queryClient.invalidateQueries({ queryKey: ["announcements"] });
    },
    onError: (err: Error) => setMessage(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.adminDeleteAnnouncement(id),
    onSuccess: () => {
      setMessage("Deleted.");
      queryClient.invalidateQueries({ queryKey: ["admin", "announcements"] });
    },
    onError: (err: Error) => setMessage(err.message),
  });

  return (
    <div>
      <h1>Manage Announcements</h1>
      <div className="admin-form card">
        <h2>{editing ? "Edit" : "New"} Announcement</h2>
        <label>Title<input className="field" value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} /></label>
        <label>Body<textarea className="field section-textarea" rows={5} value={form.body} onChange={(e) => setForm({ ...form, body: e.target.value })} /></label>
        <label className="published-toggle"><input type="checkbox" checked={form.pinned} onChange={(e) => setForm({ ...form, pinned: e.target.checked })} /> Pinned</label>
        <label className="published-toggle"><input type="checkbox" checked={form.published} onChange={(e) => setForm({ ...form, published: e.target.checked })} /> Published</label>
        <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.75rem" }}>
          <button className="btn btn-primary" disabled={saveMutation.isPending} onClick={() => saveMutation.mutate()}>
            {saveMutation.isPending ? "Saving…" : "Save"}
          </button>
          {editing && <button className="btn" onClick={() => { setEditing(null); setForm(emptyForm()); }}>Cancel</button>}
        </div>
      </div>

      {isLoading && <p className="text-muted">Loading…</p>}
      <ul className="admin-list">
        {items?.map((a) => (
          <li key={a.id} className="card admin-list-item">
            <div><strong>{a.title}</strong>{a.pinned && " 📌"}</div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button className="btn btn-small" onClick={() => { setEditing(a); setForm({ title: a.title, body: a.body, pinned: a.pinned, published: a.published }); }}>Edit</button>
              <button className="btn btn-danger btn-small" onClick={() => deleteMutation.mutate(a.id)}>Delete</button>
            </div>
          </li>
        ))}
      </ul>
      {message && <p className="admin-message">{message}</p>}
    </div>
  );
}
