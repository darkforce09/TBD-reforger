import { useState } from "react";
import type { PageSection } from "../api/client";
import { MarkdownRenderer } from "./MarkdownRenderer";
import "./SectionEditor.css";

type Props = {
  section: PageSection;
  onChange: (section: PageSection) => void;
  onDelete: (id: string) => void;
};

export function SectionEditor({ section, onChange, onDelete }: Props) {
  const [preview, setPreview] = useState(false);

  return (
    <div className="section-editor card">
      <div className="section-editor-header">
        <input
          className="field"
          placeholder="Section key"
          value={section.sectionKey}
          onChange={(e) =>
            onChange({ ...section, sectionKey: e.target.value })
          }
        />
        <input
          className="field"
          placeholder="Heading (optional)"
          value={section.heading}
          onChange={(e) => onChange({ ...section, heading: e.target.value })}
        />
        <input
          className="field field-small"
          type="number"
          placeholder="Order"
          value={section.sortOrder}
          onChange={(e) =>
            onChange({ ...section, sortOrder: Number(e.target.value) })
          }
        />
        <button className="btn" onClick={() => setPreview((p) => !p)}>
          {preview ? "Edit" : "Preview"}
        </button>
        <button
          className="btn btn-danger"
          onClick={() => onDelete(section.id)}
        >
          Delete
        </button>
      </div>
      {preview ? (
        <MarkdownRenderer content={section.content} />
      ) : (
        <textarea
          className="section-textarea"
          value={section.content}
          onChange={(e) => onChange({ ...section, content: e.target.value })}
          rows={10}
        />
      )}
    </div>
  );
}
