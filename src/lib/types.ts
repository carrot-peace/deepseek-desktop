export type ChatRole = "system" | "user" | "assistant" | "tool";

export type ChatModel = "deepseek-v4-pro" | "deepseek-v4-flash";
export type ThinkingMode = "off" | "high" | "max";
export type SearchProvider = "tavily" | "brave";

export interface Conversation {
  id: string;
  title: string;
  model: ChatModel;
  thinkingMode: ThinkingMode;
  searchEnabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ChatMessage {
  id: string;
  conversationId: string;
  role: ChatRole;
  content: string;
  reasoningContent?: string | null;
  toolCallsJson?: string | null;
  toolResultJson?: string | null;
  createdAt: string;
}

export interface AppSettings {
  deepseekBaseUrl: string;
  defaultModel: ChatModel;
  defaultThinkingMode: ThinkingMode;
  defaultSearchEnabled: boolean;
  showReasoningContent: boolean;
  searchProvider: SearchProvider;
}

export interface SendMessageRequest {
  conversationId: string;
  content: string;
  model: ChatModel;
  thinkingMode: ThinkingMode;
  searchEnabled: boolean;
}

export interface ContentDeltaEvent {
  conversationId: string;
  messageId: string;
  delta: string;
}

export interface ReasoningDeltaEvent {
  conversationId: string;
  messageId: string;
  delta: string;
}

export interface ChatErrorEvent {
  conversationId: string;
  messageId?: string;
  error: string;
}

export interface SearchResult {
  title: string;
  url: string;
  snippet: string;
  publishedAt?: string;
  source?: string;
  sourceDomain?: string;
  rawContent?: string;
  score?: number;
  sourceQuery?: string;
  sourceCategory?: string;
}
