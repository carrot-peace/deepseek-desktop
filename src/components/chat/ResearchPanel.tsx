import { Download, Pause, Play, RotateCcw, Square, XCircle } from "lucide-react";
import type { ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useChatStore } from "../../stores/chatStore";
import type { ResearchPlan, ResearchTaskDetail } from "../../lib/types";

const statusLabel: Record<string, string> = {
  draft: "待确认",
  running: "研究中",
  paused: "已暂停",
  completed: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

const policyLabel: Record<string, string> = {
  web: "全网",
  includeDomains: "仅指定站点",
  preferDomains: "优先指定站点",
};

export function ResearchPanel() {
  const currentResearchTaskId = useChatStore((state) => state.currentResearchTaskId);
  const detail = useChatStore((state) =>
    currentResearchTaskId ? state.researchTasks[currentResearchTaskId] : undefined,
  );
  const progress = useChatStore((state) =>
    currentResearchTaskId ? state.researchProgress[currentResearchTaskId] : undefined,
  );
  const startResearchTask = useChatStore((state) => state.startResearchTask);
  const pauseResearchTask = useChatStore((state) => state.pauseResearchTask);
  const resumeResearchTask = useChatStore((state) => state.resumeResearchTask);
  const cancelResearchTask = useChatStore((state) => state.cancelResearchTask);
  const exportResearchTask = useChatStore((state) => state.exportResearchTask);
  const isGenerating = useChatStore((state) => state.isGenerating);

  if (!detail) return null;

  const plan = parsePlan(detail.task.planJson);
  const percentage = progress
    ? Math.round((progress.completedSteps / Math.max(progress.totalSteps, 1)) * 100)
    : detail.task.status === "completed"
      ? 100
      : 0;

  return (
    <section className="rounded-lg border border-[var(--border-subtle)] bg-[var(--surface)] text-[var(--text-primary)]">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--border-subtle)] px-4 py-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-semibold">
            <span>Deep Research</span>
            <span className="rounded-md bg-[var(--surface-muted)] px-2 py-0.5 text-xs text-[var(--text-secondary)]">
              {statusLabel[detail.task.status] ?? detail.task.status}
            </span>
          </div>
          <h3 className="mt-1 truncate text-sm text-[var(--text-secondary)]">{detail.task.topic}</h3>
        </div>
        <div className="flex items-center gap-1">
          {detail.task.status === "draft" ? (
            <IconButton title="开始研究" disabled={isGenerating} onClick={() => void startResearchTask(detail.task.id)}>
              <Play size={15} fill="currentColor" />
            </IconButton>
          ) : null}
          {detail.task.status === "running" ? (
            <IconButton title="暂停研究" onClick={() => void pauseResearchTask(detail.task.id)}>
              <Pause size={15} />
            </IconButton>
          ) : null}
          {detail.task.status === "paused" ? (
            <IconButton title="继续研究" onClick={() => void resumeResearchTask(detail.task.id)}>
              <RotateCcw size={15} />
            </IconButton>
          ) : null}
          {["draft", "running", "paused"].includes(detail.task.status) ? (
            <IconButton title="取消研究" onClick={() => void cancelResearchTask(detail.task.id)}>
              <Square size={13} fill="currentColor" />
            </IconButton>
          ) : null}
          <IconButton title="导出 Markdown" onClick={() => void exportResearchTask(detail.task.id)}>
            <Download size={15} />
          </IconButton>
        </div>
      </header>

      <div className="space-y-4 px-4 py-4">
        <div>
          <div className="mb-2 flex items-center justify-between text-xs text-[var(--text-secondary)]">
            <span>{progress?.message ?? "计划已准备"}</span>
            <span>{percentage}%</span>
          </div>
          <div className="h-1.5 overflow-hidden rounded-full bg-[var(--surface-muted)]">
            <div
              className="h-full bg-[var(--text-primary)] transition-all"
              style={{ width: `${Math.max(4, percentage)}%` }}
            />
          </div>
        </div>

        {plan ? <ResearchPlanSummary detail={detail} plan={plan} /> : null}

        {detail.activities.length > 0 ? (
          <details open={detail.task.status !== "draft"}>
            <summary className="cursor-pointer text-sm font-medium">活动历史</summary>
            <ol className="mt-2 space-y-2 text-sm text-[var(--text-secondary)]">
              {detail.activities.map((activity) => (
                <li key={activity.id} className="flex gap-2">
                  <span className="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--text-muted)]" />
                  <span>
                    <span className="text-[var(--text-primary)]">{activity.title}</span>
                    {activity.detail ? <span> - {activity.detail}</span> : null}
                  </span>
                </li>
              ))}
            </ol>
          </details>
        ) : null}

        {detail.sources.length > 0 ? (
          <details open>
            <summary className="cursor-pointer text-sm font-medium">来源 ({detail.sources.length})</summary>
            <div className="mt-2 grid gap-2">
              {detail.sources.slice(0, 12).map((source) => (
                <a
                  key={source.id}
                  href={source.url}
                  target="_blank"
                  rel="noreferrer"
                  className="block rounded-lg border border-[var(--border-subtle)] px-3 py-2 text-sm hover:bg-[var(--surface-hover)]"
                >
                  <div className="font-medium">[S{source.sourceNumber}] {source.title}</div>
                  <div className="mt-1 truncate text-xs text-[var(--text-secondary)]">{source.url}</div>
                </a>
              ))}
            </div>
          </details>
        ) : null}

        {detail.task.report ? (
          <details>
            <summary className="cursor-pointer text-sm font-medium">报告预览</summary>
            <div className="markdown-body mt-3 max-h-80 overflow-y-auto rounded-lg border border-[var(--border-subtle)] p-3 text-sm">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{detail.task.report}</ReactMarkdown>
            </div>
          </details>
        ) : null}

        {detail.task.error ? (
          <div className="flex items-start gap-2 rounded-lg bg-[var(--danger-bg)] px-3 py-2 text-sm text-[var(--danger)]">
            <XCircle size={16} className="mt-0.5 shrink-0" />
            <span>{detail.task.error}</span>
          </div>
        ) : null}
      </div>
    </section>
  );
}

function ResearchPlanSummary({ detail, plan }: { detail: ResearchTaskDetail; plan: ResearchPlan }) {
  const domains = safeDomains(detail.task.domainsJson);

  return (
    <div className="space-y-3">
      <div className="grid gap-3 sm:grid-cols-3">
        <Metric label="来源策略" value={policyLabel[detail.task.sourcePolicy] ?? detail.task.sourcePolicy} />
        <Metric label="最大轮次" value={String(plan.depthBudget?.maxRounds ?? 4)} />
        <Metric label="来源上限" value={String(plan.depthBudget?.sourceLimit ?? 60)} />
      </div>
      {domains.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {domains.map((domain) => (
            <span key={domain} className="rounded-md bg-[var(--surface-muted)] px-2 py-1 text-xs text-[var(--text-secondary)]">
              {domain}
            </span>
          ))}
        </div>
      ) : null}
      <div>
        <h4 className="text-sm font-medium">关键问题</h4>
        <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-[var(--text-secondary)]">
          {plan.keyQuestions.slice(0, 6).map((question) => (
            <li key={question}>{question}</li>
          ))}
        </ul>
      </div>
      <div>
        <h4 className="text-sm font-medium">初始查询</h4>
        <div className="mt-2 flex flex-wrap gap-2">
          {plan.initialQueries.slice(0, 8).map((query) => (
            <span key={query.query} className="rounded-md border border-[var(--border-subtle)] px-2 py-1 text-xs text-[var(--text-secondary)]">
              {query.query}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-[var(--border-subtle)] px-3 py-2">
      <div className="text-xs text-[var(--text-muted)]">{label}</div>
      <div className="mt-1 text-sm font-medium">{value}</div>
    </div>
  );
}

function IconButton({
  title,
  disabled,
  onClick,
  children,
}: {
  title: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      className="grid h-8 w-8 place-items-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)] disabled:opacity-50"
      title={title}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function parsePlan(value: string): ResearchPlan | null {
  try {
    return JSON.parse(value) as ResearchPlan;
  } catch {
    return null;
  }
}

function safeDomains(value: string): string[] {
  try {
    return JSON.parse(value) as string[];
  } catch {
    return [];
  }
}
