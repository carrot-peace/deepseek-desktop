import { ChevronDown, Globe2, Menu, MessageSquarePlus, PanelLeft, Settings } from "lucide-react";
import { useChatStore } from "../../stores/chatStore";
import { useSettingsStore } from "../../stores/settingsStore";
import type { ChatModel, ThinkingMode } from "../../lib/types";

interface TopBarProps {
  isSidebarPinned: boolean;
  onToggleSidebar: () => void;
  onOpenMobileSidebar: () => void;
}

export function TopBar({ isSidebarPinned, onToggleSidebar, onOpenMobileSidebar }: TopBarProps) {
  const conversations = useChatStore((state) => state.conversations);
  const currentConversationId = useChatStore((state) => state.currentConversationId);
  const updateConversation = useChatStore((state) => state.updateConversation);
  const createConversation = useChatStore((state) => state.createConversation);
  const setSettingsOpen = useSettingsStore((state) => state.setSettingsOpen);
  const conversation = conversations.find((item) => item.id === currentConversationId);

  const patch = (changes: Partial<NonNullable<typeof conversation>>) => {
    if (!conversation) return;
    void updateConversation({ ...conversation, ...changes, updatedAt: new Date().toISOString() });
  };

  return (
    <header className="flex h-14 shrink-0 items-center gap-2 bg-[var(--main-bg)] px-3 sm:px-4">
      <button
        className="grid h-10 w-10 place-items-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] sm:hidden"
        title="打开侧边栏"
        onClick={onOpenMobileSidebar}
      >
        <Menu size={18} />
      </button>

      <button
        className="hidden h-10 w-10 place-items-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] sm:grid"
        title={isSidebarPinned ? "收起侧边栏" : "展开侧边栏"}
        onClick={onToggleSidebar}
      >
        <PanelLeft size={18} />
      </button>

      {!isSidebarPinned ? (
        <button
          className="hidden h-10 w-10 place-items-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] sm:grid"
          title="新对话"
          onClick={() => void createConversation()}
        >
          <MessageSquarePlus size={18} />
        </button>
      ) : null}

      <div className="relative min-w-0 flex-1 sm:flex-none">
        <select
          className="h-10 max-w-[220px] appearance-none truncate rounded-lg bg-transparent py-0 pl-3 pr-8 text-base font-semibold outline-none hover:bg-[var(--surface-hover)] disabled:opacity-70"
          value={conversation?.model ?? "deepseek-v4-pro"}
          disabled={!conversation}
          onChange={(event) => patch({ model: event.target.value as ChatModel })}
          title="选择模型"
        >
          <option value="deepseek-v4-pro">deepseek-v4-pro</option>
          <option value="deepseek-v4-flash">deepseek-v4-flash</option>
        </select>
        <ChevronDown
          size={16}
          className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[var(--text-secondary)]"
        />
      </div>

      <select
        className="hidden h-9 rounded-lg bg-transparent px-2 text-sm text-[var(--text-secondary)] outline-none hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] md:block"
        value={conversation?.thinkingMode ?? "off"}
        disabled={!conversation}
        onChange={(event) => patch({ thinkingMode: event.target.value as ThinkingMode })}
        title="推理模式"
      >
        <option value="off">Normal</option>
        <option value="high">Reasoning</option>
        <option value="max">Deep Reasoning</option>
      </select>

      <button
        className={`grid h-10 w-10 place-items-center rounded-lg text-sm ${
          conversation?.searchEnabled
            ? "bg-[var(--text-primary)] text-[var(--main-bg)]"
            : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
        }`}
        disabled={!conversation}
        onClick={() => patch({ searchEnabled: !conversation?.searchEnabled })}
        title="联网搜索"
      >
        <Globe2 size={16} />
      </button>

      <button
        className="grid h-10 w-10 place-items-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
        title="设置"
        onClick={() => setSettingsOpen(true)}
      >
        <Settings size={18} />
      </button>
    </header>
  );
}
