import ReactMarkdown from "react-markdown";

type Props = {
  content: string;
};

export function MarkdownRenderer({ content }: Props) {
  return (
    <div className="markdown">
      <ReactMarkdown>{content}</ReactMarkdown>
    </div>
  );
}
