import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { MarkdownRenderer } from "../components/MarkdownRenderer";

export function AnnouncementsPage() {
  const { data, isLoading } = useQuery({
    queryKey: ["announcements"],
    queryFn: () => api.listAnnouncements(),
  });

  return (
    <div>
      <header className="page-header">
        <h1 className="page-title">Announcements</h1>
        <p className="page-subtitle">Latest news and updates from TBD Event.</p>
      </header>

      {isLoading && <p className="text-muted">Loading…</p>}
      {data?.map((a) => (
        <article key={a.id} className="card" style={{ marginBottom: "1rem" }}>
          {a.pinned && <span className="badge badge--pinned">Pinned</span>}
          <h2 style={{ margin: "0.25rem 0 0.5rem", fontSize: "1.15rem" }}>{a.title}</h2>
          {a.publishedAt && (
            <p className="text-muted" style={{ fontSize: "0.8rem", margin: "0 0 0.5rem" }}>
              {new Date(a.publishedAt).toLocaleDateString()}
            </p>
          )}
          <MarkdownRenderer content={a.body} />
        </article>
      ))}
      {data?.length === 0 && !isLoading && (
        <p className="text-muted">No announcements yet.</p>
      )}
    </div>
  );
}
