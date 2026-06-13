import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type PageSection, type UpsertSectionInput } from "../../api/client";
import { SectionEditor } from "../../components/SectionEditor";

const PAGE_SLUGS = [
  { slug: "rules", label: "Rules" },
  { slug: "compliance", label: "Compliance" },
  { slug: "server-info", label: "Server" },
  { slug: "mods", label: "Mods" },
];

export function AdminContentTab() {
  const [selectedSlug, setSelectedSlug] = useState(PAGE_SLUGS[0].slug);
  const [sections, setSections] = useState<PageSection[]>([]);
  const [newSectionIds, setNewSectionIds] = useState<Set<string>>(new Set());
  const [published, setPublished] = useState(true);
  const [title, setTitle] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const queryClient = useQueryClient();

  const pageQuery = useQuery({
    queryKey: ["admin", "pages", selectedSlug],
    queryFn: () => api.getAdminPage(selectedSlug),
  });

  useEffect(() => {
    if (pageQuery.data) {
      setSections(pageQuery.data.sections);
      setPublished(pageQuery.data.published);
      setTitle(pageQuery.data.title);
      setNewSectionIds(new Set());
    }
  }, [pageQuery.data]);

  const saveMutation = useMutation({
    mutationFn: async () => {
      await api.updatePage(selectedSlug, { title, published });
      const payload: UpsertSectionInput[] = sections.map((s) => {
        const input: UpsertSectionInput = {
          sectionKey: s.sectionKey,
          heading: s.heading,
          content: s.content,
          sortOrder: s.sortOrder,
        };
        if (!newSectionIds.has(s.id)) input.id = s.id;
        return input;
      });
      return api.upsertSections(selectedSlug, payload);
    },
    onSuccess: async () => {
      setMessage("Saved successfully.");
      await queryClient.invalidateQueries({ queryKey: ["pages", selectedSlug] });
      await queryClient.invalidateQueries({ queryKey: ["admin", "pages", selectedSlug] });
    },
    onError: (err: Error) => setMessage(err.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteSection(id),
    onSuccess: (_, id) => {
      setSections((prev) => prev.filter((s) => s.id !== id));
      setMessage("Section deleted.");
    },
    onError: (err: Error) => setMessage(err.message),
  });

  const addSection = () => {
    const id = crypto.randomUUID();
    setNewSectionIds((prev) => new Set(prev).add(id));
    setSections((prev) => [
      ...prev,
      { id, sectionKey: `section-${Date.now()}`, heading: "New section", content: "", sortOrder: prev.length },
    ]);
  };

  return (
    <div className="admin-layout">
      <aside className="admin-sidebar card">
        <h2>Pages</h2>
        <ul className="admin-page-list">
          {PAGE_SLUGS.map((page) => (
            <li key={page.slug}>
              <button
                className={selectedSlug === page.slug ? "admin-page-btn active" : "admin-page-btn"}
                onClick={() => setSelectedSlug(page.slug)}
              >
                {page.label}
              </button>
            </li>
          ))}
        </ul>
      </aside>
      <section className="admin-content">
        <div className="admin-toolbar">
          <h1>Editing: {title || selectedSlug}</h1>
          <div className="admin-toolbar-actions">
            <label className="published-toggle">
              <input type="checkbox" checked={published} onChange={(e) => setPublished(e.target.checked)} />
              Published
            </label>
            <button className="btn btn-primary" disabled={saveMutation.isPending} onClick={() => saveMutation.mutate()}>
              {saveMutation.isPending ? "Saving…" : "Save changes"}
            </button>
          </div>
        </div>
        <div className="admin-meta card">
          <label>
            Page title
            <input className="field" value={title} onChange={(e) => setTitle(e.target.value)} />
          </label>
        </div>
        {pageQuery.isLoading ? (
          <div className="loading">Loading page…</div>
        ) : (
          <>
            {sections.map((section) => (
              <SectionEditor
                key={section.id}
                section={section}
                onChange={(updated) => setSections((prev) => prev.map((s) => (s.id === updated.id ? updated : s)))}
                onDelete={(id) => {
                  if (newSectionIds.has(id)) {
                    setSections((prev) => prev.filter((s) => s.id !== id));
                    setNewSectionIds((prev) => { const n = new Set(prev); n.delete(id); return n; });
                    return;
                  }
                  deleteMutation.mutate(id);
                }}
              />
            ))}
            <button className="btn" onClick={addSection}>Add section</button>
          </>
        )}
        {message && <p className="admin-message">{message}</p>}
      </section>
    </div>
  );
}
