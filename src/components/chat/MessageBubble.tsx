import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Clipboard, Sparkles } from "lucide-react";
import clsx from "clsx";
import type { ChatMessage } from "../../lib/types";
import { CodeBlock } from "./CodeBlock";
import { ReasoningBlock } from "./ReasoningBlock";
import { useSettingsStore } from "../../stores/settingsStore";

export function MessageBubble({ message }: { message: ChatMessage }) {
  const showReasoningContent = useSettingsStore((state) => state.settings.showReasoningContent);
  const isUser = message.role === "user";

  return (
    <article className={clsx("group flex w-full", isUser ? "justify-end" : "justify-start")}>
      <div
        className={clsx(
          isUser
            ? "max-w-[78%] rounded-3xl bg-[var(--user-bubble)] px-5 py-3 text-[15px] leading-6 text-[var(--text-primary)] sm:max-w-[70%]"
            : "flex w-full max-w-full gap-4 text-[15px] leading-6 text-[var(--text-primary)]",
        )}
      >
        {!isUser ? (
          <div className="mt-1 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-[var(--assistant-avatar)] text-[var(--assistant-avatar-fg)]">
            <Sparkles size={16} />
          </div>
        ) : null}

        <div className={clsx(!isUser && "min-w-0 flex-1")}>
          {!isUser && message.reasoningContent && showReasoningContent ? (
            <ReasoningBlock content={message.reasoningContent} />
          ) : null}

          <div className="markdown-body text-sm">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                code(props) {
                  const { children, className } = props;
                  const match = /language-(\w+)/.exec(className ?? "");
                  return match ? (
                    <CodeBlock language={match[1]} value={String(children).replace(/\n$/, "")} />
                  ) : (
                    <code className="rounded-md bg-[var(--inline-code-bg)] px-1.5 py-0.5">{children}</code>
                  );
                },
              }}
            >
              {message.content || (message.role === "assistant" ? " " : message.content)}
            </ReactMarkdown>
          </div>
          {!isUser ? (
            <button
              className="mt-2 grid h-8 w-8 place-items-center rounded-lg text-[var(--text-muted)] opacity-60 hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] group-hover:opacity-100"
              title="复制消息"
              onClick={() => void navigator.clipboard.writeText(message.content)}
            >
              <Clipboard size={15} />
            </button>
          ) : null}
        </div>
      </div>
    </article>
  );
}
