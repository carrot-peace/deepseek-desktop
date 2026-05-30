import { ChatInput } from "./ChatInput";
import { MessageList } from "./MessageList";
import { useChatStore } from "../../stores/chatStore";

export function ChatView() {
  const currentConversationId = useChatStore((state) => state.currentConversationId);
  const error = useChatStore((state) => state.error);

  return (
    <div className="relative flex min-h-0 flex-1 flex-col bg-[var(--main-bg)]">
      <div className="min-h-0 flex-1">
        {currentConversationId ? (
          <MessageList />
        ) : (
          <div className="grid h-full place-items-center px-6 text-center">
            <div>
              <h2 className="text-3xl font-semibold tracking-tight">有什么可以帮忙的？</h2>
              <p className="mt-3 text-sm text-[var(--text-secondary)]">先在设置中填写 API Key，然后开始对话。</p>
            </div>
          </div>
        )}
      </div>
      {error ? (
        <div className="mx-auto mb-2 w-full max-w-3xl px-4">
          <div className="rounded-xl bg-[var(--danger-bg)] px-4 py-2 text-sm text-[var(--danger)]">{error}</div>
        </div>
      ) : null}
      <ChatInput disabled={!currentConversationId} />
    </div>
  );
}
