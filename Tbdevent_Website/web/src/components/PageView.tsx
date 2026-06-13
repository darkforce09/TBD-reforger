import { useQuery } from "@tanstack/react-query";
import { api } from "../api/client";
import { MarkdownRenderer } from "./MarkdownRenderer";

type Props = {
  slug: string;
  subtitle?: string;
};

export function PageView({ slug, subtitle }: Props) {
  const { data, isLoading, error } = useQuery({
    queryKey: ["pages", slug],
    queryFn: () => api.getPage(slug),
  });

  if (isLoading) return <div className="loading">Loading…</div>;
  if (error) return <div className="error">Failed to load page.</div>;
  if (!data) return null;

  return (
    <div>
      <header className="page-header">
        <h1 className="page-title">{data.title}</h1>
        {subtitle && <p className="page-subtitle">{subtitle}</p>}
      </header>
      {data.sections.map((section) => (
        <section
          key={section.id}
          className={
            section.heading ? "page-section" : "page-section page-section--intro"
          }
        >
          {section.heading && (
            <h2 className="section-title">{section.heading}</h2>
          )}
          <div
            className={
              section.sectionKey === "pvp-note" ? "info-box" : "section-content"
            }
          >
            <MarkdownRenderer content={section.content} />
          </div>
        </section>
      ))}
    </div>
  );
}
