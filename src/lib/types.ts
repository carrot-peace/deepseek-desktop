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

export type ResearchStatus = "draft" | "running" | "paused" | "completed" | "failed" | "cancelled";
export type ResearchSourcePolicy = "web" | "includeDomains" | "preferDomains";

export interface PrepareResearchTaskRequest {
  conversationId: string;
  prompt: string;
  model: ChatModel;
  sourcePolicy: ResearchSourcePolicy;
  domains: string[];
}

export interface StartResearchTaskRequest {
  taskId: string;
  model: ChatModel;
}

export interface ResearchDepthBudget {
  maxRounds?: number;
  queriesPerRound?: number;
  sourceLimit?: number;
}

export interface ResearchPlan {
  title: string;
  goal: string;
  audience?: string | null;
  keyQuestions: string[];
  mustHave: string[];
  initialQueries: PlannedSearchQuery[];
  successCriteria: string[];
  sourcePolicy?: ResearchSourcePolicy | null;
  domains: string[];
  depthBudget?: ResearchDepthBudget | null;
}

export interface PlannedSearchQuery {
  query: string;
  topic?: string | null;
  searchDepth?: string | null;
  maxResults?: number | null;
  includeDomains: string[];
  excludeDomains: string[];
  startDate?: string | null;
  endDate?: string | null;
  includeRawContent?: boolean | null;
}

export interface ResearchTask {
  id: string;
  conversationId: string;
  userMessageId: string;
  assistantMessageId?: string | null;
  topic: string;
  status: ResearchStatus;
  sourcePolicy: ResearchSourcePolicy;
  domainsJson: string;
  planJson: string;
  report: string;
  error?: string | null;
  createdAt: string;
  updatedAt: string;
  completedAt?: string | null;
}

export interface ResearchSource {
  id: string;
  taskId: string;
  sourceNumber: number;
  title: string;
  url: string;
  snippet: string;
  publishedAt?: string | null;
  sourceDomain?: string | null;
  rawContent?: string | null;
  score?: number | null;
  sourceQuery?: string | null;
  createdAt: string;
}

export interface ResearchActivity {
  id: string;
  taskId: string;
  activityType: string;
  title: string;
  detail?: string | null;
  createdAt: string;
}

export interface ResearchTaskDetail {
  task: ResearchTask;
  sources: ResearchSource[];
  activities: ResearchActivity[];
}

export interface PrepareResearchTaskResponse {
  detail: ResearchTaskDetail;
  userMessage: ChatMessage;
}

export interface ResearchProgressEvent {
  taskId: string;
  conversationId: string;
  status: ResearchStatus;
  phase: string;
  completedSteps: number;
  totalSteps: number;
  message: string;
}

export interface ResearchReportDeltaEvent {
  taskId: string;
  conversationId: string;
  messageId: string;
  delta: string;
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
