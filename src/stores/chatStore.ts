import { create } from "zustand";
import { events, commands } from "../lib/tauri";
import type {
  ChatMessage,
  Conversation,
  ResearchActivity,
  ResearchProgressEvent,
  ResearchSource,
  ResearchSourcePolicy,
  ResearchTaskDetail,
} from "../lib/types";

interface ChatState {
  conversations: Conversation[];
  currentConversationId?: string;
  messages: ChatMessage[];
  researchTasks: Record<string, ResearchTaskDetail>;
  currentResearchTaskId?: string;
  researchProgress: Record<string, ResearchProgressEvent>;
  isGenerating: boolean;
  error?: string;
  loadConversations: () => Promise<void>;
  createConversation: () => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  switchConversation: (id: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
  prepareResearch: (
    content: string,
    options: { sourcePolicy: ResearchSourcePolicy; domains: string[] },
  ) => Promise<void>;
  startResearchTask: (taskId: string) => Promise<void>;
  pauseResearchTask: (taskId: string) => Promise<void>;
  resumeResearchTask: (taskId: string) => Promise<void>;
  cancelResearchTask: (taskId: string) => Promise<void>;
  exportResearchTask: (taskId: string) => Promise<void>;
  stopGeneration: () => Promise<void>;
  updateConversation: (conversation: Conversation) => Promise<void>;
  initializeEventListeners: () => Promise<void>;
}

const nowIso = () => new Date().toISOString();
const tempId = () => crypto.randomUUID();

declare global {
  var __deepseekChatListenersReady: boolean | undefined;
}

const detailMap = (details: ResearchTaskDetail[]) =>
  Object.fromEntries(details.map((detail) => [detail.task.id, detail]));

const newestTaskId = (details: ResearchTaskDetail[]) => details[0]?.task.id;

const upsertDetail = (
  tasks: Record<string, ResearchTaskDetail>,
  detail: ResearchTaskDetail,
) => ({
  ...tasks,
  [detail.task.id]: detail,
});

const appendActivity = (
  detail: ResearchTaskDetail,
  activity: ResearchActivity,
): ResearchTaskDetail => {
  if (detail.activities.some((item) => item.id === activity.id)) return detail;
  return { ...detail, activities: [...detail.activities, activity] };
};

const appendSources = (
  detail: ResearchTaskDetail,
  sources: ResearchSource[],
): ResearchTaskDetail => {
  const seen = new Set(detail.sources.map((source) => source.id));
  const next = sources.filter((source) => !seen.has(source.id));
  if (next.length === 0) return detail;
  return {
    ...detail,
    sources: [...detail.sources, ...next].sort((a, b) => a.sourceNumber - b.sourceNumber),
  };
};

const downloadMarkdown = (filename: string, content: string) => {
  const blob = new Blob([content], { type: "text/markdown;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
};

export const useChatStore = create<ChatState>((set, get) => ({
  conversations: [],
  messages: [],
  researchTasks: {},
  researchProgress: {},
  isGenerating: false,
  async loadConversations() {
    const conversations = await commands.getConversations();
    const currentConversationId = conversations[0]?.id;
    const messages = currentConversationId ? await commands.getMessages(currentConversationId) : [];
    const researchDetails = currentConversationId ? await commands.getResearchTasks(currentConversationId) : [];
    set({
      conversations,
      currentConversationId,
      messages,
      researchTasks: detailMap(researchDetails),
      currentResearchTaskId: newestTaskId(researchDetails),
      error: undefined,
    });
  },
  async createConversation() {
    const conversation = await commands.createConversation();
    set((state) => ({
      conversations: [conversation, ...state.conversations],
      currentConversationId: conversation.id,
      messages: [],
      researchTasks: {},
      currentResearchTaskId: undefined,
      researchProgress: {},
      error: undefined,
    }));
  },
  async deleteConversation(id) {
    try {
      await commands.deleteConversation(id);
      const remaining = get().conversations.filter((conversation) => conversation.id !== id);
      const nextId = get().currentConversationId === id ? remaining[0]?.id : get().currentConversationId;
      const messages = nextId ? await commands.getMessages(nextId) : [];
      const researchDetails = nextId ? await commands.getResearchTasks(nextId) : [];
      set({
        conversations: remaining,
        currentConversationId: nextId,
        messages,
        researchTasks: detailMap(researchDetails),
        currentResearchTaskId: newestTaskId(researchDetails),
        error: undefined,
      });
    } catch (error) {
      set({ error: `删除会话失败：${String(error)}` });
      throw error;
    }
  },
  async switchConversation(id) {
    const messages = await commands.getMessages(id);
    const researchDetails = await commands.getResearchTasks(id);
    set({
      currentConversationId: id,
      messages,
      researchTasks: detailMap(researchDetails),
      currentResearchTaskId: newestTaskId(researchDetails),
      error: undefined,
    });
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
  async prepareResearch(content, options) {
    const trimmed = content.trim();
    if (!trimmed || get().isGenerating) return;

    let conversation = get().conversations.find((item) => item.id === get().currentConversationId);
    if (!conversation) {
      await get().createConversation();
      conversation = get().conversations.find((item) => item.id === get().currentConversationId);
    }
    if (!conversation) return;

    set({ isGenerating: true, error: undefined });
    try {
      const response = await commands.prepareResearchTask({
        conversationId: conversation.id,
        prompt: trimmed,
        model: conversation.model,
        sourcePolicy: options.sourcePolicy,
        domains: options.domains,
      });
      const conversations = await commands.getConversations();
      set((state) => ({
        conversations,
        messages: state.messages.some((message) => message.id === response.userMessage.id)
          ? state.messages
          : [...state.messages, response.userMessage],
        researchTasks: upsertDetail(state.researchTasks, response.detail),
        currentResearchTaskId: response.detail.task.id,
        isGenerating: false,
      }));
    } catch (error) {
      set({ isGenerating: false, error: String(error) });
    }
  },
  async startResearchTask(taskId) {
    const detail = get().researchTasks[taskId];
    const conversation = get().conversations.find((item) => item.id === detail?.task.conversationId);
    if (!detail || !conversation || get().isGenerating) return;

    set({ isGenerating: true, error: undefined, currentResearchTaskId: taskId });
    try {
      const next = await commands.startResearchTask({ taskId, model: conversation.model });
      set((state) => ({
        researchTasks: upsertDetail(state.researchTasks, next),
        currentResearchTaskId: taskId,
      }));
    } catch (error) {
      set({ isGenerating: false, error: String(error) });
    }
  },
  async pauseResearchTask(taskId) {
    try {
      const detail = await commands.pauseResearchTask(taskId);
      set((state) => ({ researchTasks: upsertDetail(state.researchTasks, detail), isGenerating: false }));
    } catch (error) {
      set({ error: String(error) });
    }
  },
  async resumeResearchTask(taskId) {
    try {
      const detail = await commands.resumeResearchTask(taskId);
      set((state) => ({ researchTasks: upsertDetail(state.researchTasks, detail), isGenerating: true }));
    } catch (error) {
      set({ error: String(error) });
    }
  },
  async cancelResearchTask(taskId) {
    try {
      const detail = await commands.cancelResearchTask(taskId);
      set((state) => ({ researchTasks: upsertDetail(state.researchTasks, detail), isGenerating: false }));
    } catch (error) {
      set({ isGenerating: false, error: String(error) });
    }
  },
  async exportResearchTask(taskId) {
    try {
      const markdown = await commands.exportResearchTask(taskId);
      const detail = get().researchTasks[taskId];
      const filename = `${detail?.task.topic ?? "deep-research"}.md`.replace(/[/:*?"<>|]/g, "-");
      downloadMarkdown(filename, markdown);
    } catch (error) {
      set({ error: String(error) });
    }
  },
  async stopGeneration() {
    const currentResearchTaskId = get().currentResearchTaskId;
    const currentTask = currentResearchTaskId
      ? get().researchTasks[currentResearchTaskId]
      : undefined;
    if (currentTask && ["draft", "running", "paused"].includes(currentTask.task.status)) {
      await get().cancelResearchTask(currentTask.task.id);
      return;
    }
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
      events.onResearchProgress((progress) => {
        set((state) => {
          const detail = state.researchTasks[progress.taskId];
          const finished = ["completed", "failed", "cancelled"].includes(progress.status);
          return {
            researchProgress: { ...state.researchProgress, [progress.taskId]: progress },
            researchTasks: detail
              ? {
                  ...state.researchTasks,
                  [progress.taskId]: {
                    ...detail,
                    task: { ...detail.task, status: progress.status },
                  },
                }
              : state.researchTasks,
            isGenerating: finished ? false : state.isGenerating,
          };
        });
      }),
      events.onResearchActivity((activity) => {
        set((state) => {
          const detail = state.researchTasks[activity.taskId];
          if (!detail) return state;
          return {
            researchTasks: {
              ...state.researchTasks,
              [activity.taskId]: appendActivity(detail, activity),
            },
          };
        });
      }),
      events.onResearchSourcesDelta((sources) => {
        const taskId = sources[0]?.taskId;
        if (!taskId) return;
        set((state) => {
          const detail = state.researchTasks[taskId];
          if (!detail) return state;
          return {
            researchTasks: {
              ...state.researchTasks,
              [taskId]: appendSources(detail, sources),
            },
          };
        });
      }),
      events.onResearchReportDelta(({ taskId, delta }) => {
        set((state) => {
          const detail = state.researchTasks[taskId];
          if (!detail) return state;
          return {
            researchTasks: {
              ...state.researchTasks,
              [taskId]: {
                ...detail,
                task: { ...detail.task, report: `${detail.task.report}${delta}` },
              },
            },
          };
        });
      }),
      events.onResearchDone(() => set({ isGenerating: false })),
      events.onResearchError(({ error }) => set({ isGenerating: false, error })),
    ]);
  },
}));
