import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  ChatMessage,
  Conversation,
  ContentDeltaEvent,
  PrepareResearchTaskRequest,
  PrepareResearchTaskResponse,
  ReasoningDeltaEvent,
  ResearchActivity,
  ResearchProgressEvent,
  ResearchReportDeltaEvent,
  ResearchSource,
  ResearchTaskDetail,
  SendMessageRequest,
  StartResearchTaskRequest,
} from "./types";

export const commands = {
  getConversations: () => invoke<Conversation[]>("get_conversations"),
  createConversation: () => invoke<Conversation>("create_conversation"),
  updateConversation: (conversation: Conversation) =>
    invoke<void>("update_conversation", { conversation }),
  deleteConversation: (conversationId: string) =>
    invoke<void>("delete_conversation", { conversationId }),
  getMessages: (conversationId: string) =>
    invoke<ChatMessage[]>("get_messages", { conversationId }),
  saveMessage: (message: ChatMessage) => invoke<void>("save_message", { message }),
  getSettings: () => invoke<AppSettings>("get_settings"),
  saveSettings: (settings: AppSettings) => invoke<void>("save_settings", { settings }),
  setSecret: (key: string, value: string) => invoke<void>("set_secret", { key, value }),
  hasSecret: (key: string) => invoke<boolean>("has_secret", { key }),
  deleteSecret: (key: string) => invoke<void>("delete_secret", { key }),
  sendMessage: (request: SendMessageRequest) => invoke<void>("send_message", { request }),
  stopGeneration: (conversationId: string) =>
    invoke<void>("stop_generation", { conversationId }),
  prepareResearchTask: (request: PrepareResearchTaskRequest) =>
    invoke<PrepareResearchTaskResponse>("prepare_research_task", { request }),
  startResearchTask: (request: StartResearchTaskRequest) =>
    invoke<ResearchTaskDetail>("start_research_task", { request }),
  pauseResearchTask: (taskId: string) =>
    invoke<ResearchTaskDetail>("pause_research_task", { taskId }),
  resumeResearchTask: (taskId: string) =>
    invoke<ResearchTaskDetail>("resume_research_task", { taskId }),
  cancelResearchTask: (taskId: string) =>
    invoke<ResearchTaskDetail>("cancel_research_task", { taskId }),
  getResearchTask: (taskId: string) =>
    invoke<ResearchTaskDetail>("get_research_task", { taskId }),
  getResearchTasks: (conversationId: string) =>
    invoke<ResearchTaskDetail[]>("get_research_tasks", { conversationId }),
  exportResearchTask: (taskId: string) =>
    invoke<string>("export_research_task", { taskId }),
};

export const events = {
  onStarted: (handler: (payload: { conversationId: string; messageId: string }) => void) =>
    listen("chat:started", (event) => handler(event.payload as { conversationId: string; messageId: string })),
  onContentDelta: (handler: (payload: ContentDeltaEvent) => void) =>
    listen("chat:content-delta", (event) => handler(event.payload as ContentDeltaEvent)),
  onReasoningDelta: (handler: (payload: ReasoningDeltaEvent) => void) =>
    listen("chat:reasoning-delta", (event) => handler(event.payload as ReasoningDeltaEvent)),
  onDone: (handler: (payload: { conversationId: string; messageId: string }) => void) =>
    listen("chat:done", (event) => handler(event.payload as { conversationId: string; messageId: string })),
  onError: (handler: (payload: { conversationId: string; messageId?: string; error: string }) => void) =>
    listen("chat:error", (event) => handler(event.payload as { conversationId: string; messageId?: string; error: string })),
  onResearchProgress: (handler: (payload: ResearchProgressEvent) => void) =>
    listen("research:progress", (event) => handler(event.payload as ResearchProgressEvent)),
  onResearchActivity: (handler: (payload: ResearchActivity) => void) =>
    listen("research:activity", (event) => handler(event.payload as ResearchActivity)),
  onResearchSourcesDelta: (handler: (payload: ResearchSource[]) => void) =>
    listen("research:sources-delta", (event) => handler(event.payload as ResearchSource[])),
  onResearchReportDelta: (handler: (payload: ResearchReportDeltaEvent) => void) =>
    listen("research:report-delta", (event) => handler(event.payload as ResearchReportDeltaEvent)),
  onResearchDone: (handler: (payload: { conversationId: string; messageId: string }) => void) =>
    listen("research:done", (event) => handler(event.payload as { conversationId: string; messageId: string })),
  onResearchError: (handler: (payload: { conversationId: string; messageId?: string; error: string }) => void) =>
    listen("research:error", (event) => handler(event.payload as { conversationId: string; messageId?: string; error: string })),
};
