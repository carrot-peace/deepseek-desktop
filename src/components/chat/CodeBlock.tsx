import { Clipboard } from "lucide-react";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { oneLight } from "react-syntax-highlighter/dist/esm/styles/prism";

export function CodeBlock({ language, value }: { language: string; value: string }) {
  return (
    <div className="my-4 overflow-hidden rounded-xl border border-[var(--border-subtle)] bg-[var(--surface-muted)]">
      <div className="flex h-9 items-center justify-between border-b border-[var(--border-subtle)] px-3">
        <span className="text-xs text-[var(--text-secondary)]">{language}</span>
        <button
          className="grid h-7 w-7 place-items-center rounded-md text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
          title="复制代码"
          onClick={() => void navigator.clipboard.writeText(value)}
        >
          <Clipboard size={14} />
        </button>
      </div>
      <SyntaxHighlighter
        language={language}
        style={oneLight}
        customStyle={{ margin: 0, padding: "0.9rem", background: "var(--surface-muted)", fontSize: "13px" }}
        codeTagProps={{ style: { fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace" } }}
      >
        {value}
      </SyntaxHighlighter>
    </div>
  );
}
