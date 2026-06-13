import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../../api/client";

export function AdminRegistrationsTab() {
  const queryClient = useQueryClient();
  const [selectedEventId, setSelectedEventId] = useState("");
  const [message, setMessage] = useState<string | null>(null);

  const { data: events } = useQuery({
    queryKey: ["admin", "events"],
    queryFn: () => api.adminListEvents(),
  });

  const { data: registrations, isLoading } = useQuery({
    queryKey: ["admin", "registrations", selectedEventId],
    queryFn: () => api.adminListRegistrations(selectedEventId),
    enabled: !!selectedEventId,
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) =>
      api.adminUpdateRegistration(id, status),
    onSuccess: () => {
      setMessage("Registration updated.");
      queryClient.invalidateQueries({ queryKey: ["admin", "registrations", selectedEventId] });
    },
    onError: (err: Error) => setMessage(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.adminDeleteRegistration(id),
    onSuccess: () => {
      setMessage("Registration removed.");
      queryClient.invalidateQueries({ queryKey: ["admin", "registrations", selectedEventId] });
    },
    onError: (err: Error) => setMessage(err.message),
  });

  const exportCSV = () => {
    if (!registrations?.length) return;
    const rows = [
      ["Username", "Discord ID", "Status", "Signed up"],
      ...registrations.map((r) => [
        r.user?.username ?? "",
        r.user?.discordId ?? "",
        r.status,
        r.signedUpAt,
      ]),
    ];
    const csv = rows.map((row) => row.map((c) => `"${c}"`).join(",")).join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "registrations.csv";
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div>
      <h1>Manage Registrations</h1>
      <div className="admin-toolbar" style={{ marginBottom: "1rem" }}>
        <select
          className="field"
          value={selectedEventId}
          onChange={(e) => setSelectedEventId(e.target.value)}
          style={{ minWidth: "240px" }}
        >
          <option value="">Select event…</option>
          {events?.map((e) => (
            <option key={e.id} value={e.id}>{e.title}</option>
          ))}
        </select>
        {registrations && registrations.length > 0 && (
          <button className="btn" onClick={exportCSV}>Export CSV</button>
        )}
      </div>

      {!selectedEventId && <p className="text-muted">Select an event to view registrations.</p>}
      {isLoading && <p className="text-muted">Loading…</p>}

      {registrations && (
        <table className="admin-table">
          <thead>
            <tr>
              <th>Player</th>
              <th>Status</th>
              <th>Signed up</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {registrations.map((r) => (
              <tr key={r.id}>
                <td>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
                    {r.user?.avatarUrl && <img src={r.user.avatarUrl} alt="" className="roster-avatar" />}
                    {r.user?.username}
                  </div>
                </td>
                <td>
                  <select
                    className="field"
                    value={r.status}
                    onChange={(e) => updateMutation.mutate({ id: r.id, status: e.target.value })}
                  >
                    <option value="registered">Registered</option>
                    <option value="waitlist">Waitlist</option>
                    <option value="cancelled">Cancelled</option>
                  </select>
                </td>
                <td>{new Date(r.signedUpAt).toLocaleString()}</td>
                <td>
                  <button className="btn btn-danger btn-small" onClick={() => deleteMutation.mutate(r.id)}>
                    Remove
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {registrations?.length === 0 && selectedEventId && !isLoading && (
        <p className="text-muted">No registrations for this event.</p>
      )}
      {message && <p className="admin-message">{message}</p>}
    </div>
  );
}
