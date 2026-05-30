import { useEffect, useRef } from "react";
import { useChatStore } from "../../stores/chatStore";
import { MessageBubble } from "./MessageBubble";
import { ResearchPanel } from "./ResearchPanel";

export function MessageList() {
  const messages = useChatStore((state) => state.messages);
  const currentResearchTaskId = useChatStore((state) => state.currentResearchTaskId);
  const currentResearchTask = useChatStore((state) =>
    currentResearchTaskId ? state.researchTasks[currentResearchTaskId] : undefined,
  );
  const scrollerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const node = scrollerRef.current;
    if (!node) return;
    node.scrollTop = node.scrollHeight;
  }, [messages, currentResearchTask?.task.report, currentResearchTask?.activities.length, currentResearchTask?.sources.length]);

  if (messages.length === 0) {
    return (
      <div className="grid h-full place-items-center px-6 pb-28 text-center">
        <div className="max-w-xl">
          <h2 className="text-3xl font-semibold tracking-tight">有什么可以帮忙的？</h2>
          <p className="mt-3 text-sm text-[var(--text-secondary)]">支持 Markdown、代码块、推理内容折叠和联网搜索。</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={scrollerRef} className="h-full overflow-y-auto px-4 pb-36 pt-6">
      <div className="mx-auto flex max-w-3xl flex-col gap-7">
        {messages
          .filter((message) => message.role !== "tool")
          .map((message) => (
            <MessageBubble key={message.id} message={message} />
          ))}
        {currentResearchTask ? <ResearchPanel /> : null}
      </div>
    </div>
  );
}
