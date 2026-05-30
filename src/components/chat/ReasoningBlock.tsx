import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

export function ReasoningBlock({ content }: { content: string }) {
  const [open, setOpen] = useState(false);

  return (
    <div className="mb-4 rounded-xl border border-[var(--border-subtle)] bg-[var(--surface-muted)]">
      <button
        className="flex w-full items-center gap-2 px-3 py-2 text-left text-[13px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
        onClick={() => setOpen((value) => !value)}
      >
        {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        思考过程
      </button>
      {open ? <div className="whitespace-pre-wrap border-t border-[var(--border-subtle)] px-3 py-3 text-[13px] leading-6 text-[var(--text-secondary)]">{content}</div> : null}
    </div>
  );
}
