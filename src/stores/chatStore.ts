import { create } from "zustand";
import { events, commands } from "../lib/tauri";
import type { ChatMessage, Conversation } from "../lib/types";

interface ChatState {
  conversations: Conversation[];
  currentConversationId?: string;
  messages: ChatMessage[];
  isGenerating: boolean;
  error?: string;
  loadConversations: () => Promise<void>;
  createConversation: () => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  switchConversation: (id: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
  stopGeneration: () => Promise<void>;
  updateConversation: (conversation: Conversation) => Promise<void>;
  initializeEventListeners: () => Promise<void>;
}

const nowIso = () => new Date().toISOString();
const tempId = () => crypto.randomUUID();

declare global {
  var __deepseekChatListenersReady: boolean | undefined;
}

export const useChatStore = create<ChatState>((set, get) => ({
  conversations: [],
  messages: [],
  isGenerating: false,
  async loadConversations() {
    const conversations = await commands.getConversations();
    const currentConversationId = conversations[0]?.id;
    const messages = currentConversationId ? await commands.getMessages(currentConversationId) : [];
    set({ conversations, currentConversationId, messages, error: undefined });
  },
  async createConversation() {
    const conversation = await commands.createConversation();
    set((state) => ({
      conversations: [conversation, ...state.conversations],
      currentConversationId: conversation.id,
      messages: [],
      error: undefined,
    }));
  },
  async deleteConversation(id) {
    try {
      await commands.deleteConversation(id);
      const remaining = get().conversations.filter((conversation) => conversation.id !== id);
      const nextId = get().currentConversationId === id ? remaining[0]?.id : get().currentConversationId;
      const messages = nextId ? await commands.getMessages(nextId) : [];
      set({ conversations: remaining, currentConversationId: nextId, messages, error: undefined });
    } catch (error) {
      set({ error: `删除会话失败：${String(error)}` });
      throw error;
    }
  },
  async switchConversation(id) {
    const messages = await commands.getMessages(id);
    set({ currentConversationId: id, messages, error: undefined });
  },
  async sendMessage(content) {
    const trimmed = content.trim();
    if (!trimmed || get().isGenerating) return;

    let conversation = get().conversations.find((item) => item.id === get().currentConversationId);
    if (!conversation) {
      await get().createConversation();
      conversation = get().conversations.find((item) => item.id === get().currentConversationId);
    }
    if (!conversation) return;

    const userMessage: ChatMessage = {
      id: tempId(),
      conversationId: conversation.id,
      role: "user",
      content: trimmed,
      createdAt: nowIso(),
    };
    set((state) => ({ messages: [...state.messages, userMessage], isGenerating: true, error: undefined }));

    try {
      await commands.sendMessage({
        conversationId: conversation.id,
        content: trimmed,
        model: conversation.model,
        thinkingMode: conversation.thinkingMode,
        searchEnabled: conversation.searchEnabled,
      });
    } catch (error) {
      set({ isGenerating: false, error: String(error) });
    }
  },
  async stopGeneration() {
    const id = get().currentConversationId;
    if (!id) return;
    await commands.stopGeneration(id);
    set({ isGenerating: false });
  },
  async updateConversation(conversation) {
    await commands.updateConversation(conversation);
    set((state) => ({
      conversations: state.conversations.map((item) => (item.id === conversation.id ? conversation : item)),
    }));
  },
  async initializeEventListeners() {
    if (globalThis.__deepseekChatListenersReady) return;
    globalThis.__deepseekChatListenersReady = true;

    const ensureAssistantMessage = (conversationId: string, messageId: string) => {
      set((state) => {
        const existing = state.messages.find((message) => message.id === messageId);
        if (existing) return state;
        return {
          messages: [
            ...state.messages,
            {
              id: messageId,
              conversationId,
              role: "assistant",
              content: "",
              reasoningContent: "",
              createdAt: nowIso(),
            },
          ],
        };
      });
    };

    const appendAssistantDelta = (
      conversationId: string,
      messageId: string,
      delta: string,
      field: "content" | "reasoningContent",
    ) => {
      set((state) => {
        const existing = state.messages.find((message) => message.id === messageId);
        if (!existing) {
          return {
            messages: [
              ...state.messages,
              {
                id: messageId,
                conversationId,
                role: "assistant",
                content: field === "content" ? delta : "",
                reasoningContent: field === "reasoningContent" ? delta : "",
                createdAt: nowIso(),
              },
            ],
          };
        }
        return {
          messages: state.messages.map((message) =>
            message.id === messageId
              ? {
                  ...message,
                  [field]: `${message[field] ?? ""}${delta}`,
                }
              : message,
          ),
        };
      });
    };

    await Promise.all([
      events.onStarted(({ conversationId, messageId }) => {
        ensureAssistantMessage(conversationId, messageId);
      }),
      events.onContentDelta(({ conversationId, messageId, delta }) => {
        appendAssistantDelta(conversationId, messageId, delta, "content");
      }),
      events.onReasoningDelta(({ conversationId, messageId, delta }) => {
        appendAssistantDelta(conversationId, messageId, delta, "reasoningContent");
      }),
      events.onDone(() => set({ isGenerating: false })),
      events.onError(({ error }) => set({ isGenerating: false, error })),
    ]);
  },
}));
