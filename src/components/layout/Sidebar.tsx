import { useState } from "react";
import { MessageSquarePlus, MoreHorizontal, Settings, X } from "lucide-react";
import clsx from "clsx";
import { useChatStore } from "../../stores/chatStore";
import { useSettingsStore } from "../../stores/settingsStore";

interface SidebarProps {
  isPinned: boolean;
  isMobileOpen: boolean;
  onCloseMobile: () => void;
}

export function Sidebar({ isPinned, isMobileOpen, onCloseMobile }: SidebarProps) {
  const [pendingDeleteId, setPendingDeleteId] = useState<string | undefined>();
  const [deletingId, setDeletingId] = useState<string>();
  const conversations = useChatStore((state) => state.conversations);
  const currentConversationId = useChatStore((state) => state.currentConversationId);
  const createConversation = useChatStore((state) => state.createConversation);
  const switchConversation = useChatStore((state) => state.switchConversation);
  const deleteConversation = useChatStore((state) => state.deleteConversation);
  const setSettingsOpen = useSettingsStore((state) => state.setSettingsOpen);
  const pendingDeleteConversation = conversations.find((conversation) => conversation.id === pendingDeleteId);

  const confirmDelete = async () => {
    if (!pendingDeleteId) return;
    const conversationId = pendingDeleteId;
    setDeletingId(conversationId);
    try {
      await deleteConversation(conversationId);
      setPendingDeleteId(undefined);
    } catch {
      // The store surfaces the failure in the chat error area.
    } finally {
      setDeletingId(undefined);
    }
  };

  const startConversation = async () => {
    await createConversation();
    onCloseMobile();
  };

  const pickConversation = async (conversationId: string) => {
    setPendingDeleteId(undefined);
    await switchConversation(conversationId);
    onCloseMobile();
  };

  return (
    <>
      <aside
        className={clsx(
          "fixed inset-y-0 left-0 z-40 flex w-[286px] shrink-0 flex-col bg-[var(--sidebar-bg)] text-[var(--text-primary)] transition-transform duration-200 ease-out sm:static sm:z-auto sm:transition-[width,opacity]",
          isMobileOpen ? "translate-x-0 shadow-2xl" : "-translate-x-full sm:translate-x-0",
          isPinned ? "sm:w-[286px] sm:opacity-100" : "sm:w-0 sm:overflow-hidden sm:opacity-0",
        )}
      >
        <div className="flex h-14 items-center gap-2 px-3">
          <button
            className="grid h-9 w-9 place-items-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--sidebar-hover)] hover:text-[var(--text-primary)] sm:hidden"
            title="关闭侧边栏"
            onClick={onCloseMobile}
          >
            <X size={18} />
          </button>
          <button
            className="flex h-10 min-w-0 flex-1 items-center gap-2 rounded-lg px-3 text-sm font-medium hover:bg-[var(--sidebar-hover)]"
            onClick={() => void startConversation()}
          >
            <MessageSquarePlus size={17} />
            <span className="truncate">新对话</span>
          </button>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
          {conversations.map((conversation) => (
            <div
              key={conversation.id}
              className={clsx(
                "group mb-0.5 flex items-center gap-1 rounded-lg px-2 py-2 text-sm",
                conversation.id === currentConversationId
                  ? "bg-[var(--sidebar-active)]"
                  : "hover:bg-[var(--sidebar-hover)]",
              )}
            >
              <button
                className="min-w-0 flex-1 text-left"
                onClick={() => void pickConversation(conversation.id)}
              >
                <div className="truncate text-[13px] font-medium">{conversation.title}</div>
              </button>
              <button
                className={clsx(
                  "grid h-8 w-8 shrink-0 place-items-center rounded-md text-[var(--text-muted)] opacity-70 hover:bg-[var(--sidebar-action)] hover:text-[var(--danger)] focus:opacity-100 group-hover:opacity-100 sm:opacity-0",
                  deletingId === conversation.id && "cursor-wait opacity-60",
                )}
                title="删除会话"
                disabled={deletingId === conversation.id}
                onClick={(event) => {
                  event.stopPropagation();
                  setPendingDeleteId(conversation.id);
                }}
              >
                <MoreHorizontal size={16} />
              </button>
            </div>
          ))}
        </div>

        <div className="p-3">
          <button
            className="flex h-10 w-full items-center gap-2 rounded-lg px-3 text-sm hover:bg-[var(--sidebar-hover)]"
            onClick={() => {
              setSettingsOpen(true);
              onCloseMobile();
            }}
          >
            <Settings size={17} />
            设置
          </button>
        </div>
      </aside>

      {pendingDeleteConversation ? (
        <div className="fixed inset-0 z-50 grid place-items-center bg-black/35 px-4">
          <div className="w-full max-w-sm rounded-2xl border border-[var(--border-subtle)] bg-[var(--surface)] p-4 text-[var(--text-primary)] shadow-xl">
            <div className="flex items-center justify-between gap-3">
              <h2 className="text-base font-semibold">删除对话？</h2>
              <button
                className="grid h-8 w-8 place-items-center rounded-md text-[var(--text-muted)] hover:bg-[var(--surface-muted)]"
                onClick={() => setPendingDeleteId(undefined)}
                disabled={deletingId === pendingDeleteConversation.id}
                title="取消"
              >
                <X size={16} />
              </button>
            </div>
            <p className="mt-3 text-sm leading-6 text-[var(--text-secondary)]">
              将删除“{pendingDeleteConversation.title}”及其中的聊天记录。此操作无法撤销。
            </p>
            <div className="mt-5 flex justify-end gap-2">
              <button
                className="h-9 rounded-lg px-3 text-sm hover:bg-[var(--surface-muted)]"
                onClick={() => setPendingDeleteId(undefined)}
                disabled={deletingId === pendingDeleteConversation.id}
              >
                取消
              </button>
              <button
                className="h-9 rounded-lg bg-[var(--danger)] px-3 text-sm font-medium text-white disabled:cursor-wait disabled:opacity-70"
                onClick={() => void confirmDelete()}
                disabled={deletingId === pendingDeleteConversation.id}
              >
                {deletingId === pendingDeleteConversation.id ? "删除中" : "确认删除"}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
