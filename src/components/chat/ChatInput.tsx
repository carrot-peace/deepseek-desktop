import { useEffect, useRef, useState } from "react";
import { Microscope, Send, Square } from "lucide-react";
import { useChatStore } from "../../stores/chatStore";
import type { ResearchSourcePolicy } from "../../lib/types";

const TEXTAREA_LINE_HEIGHT = 24;
const TEXTAREA_VERTICAL_PADDING = 16;
const MAX_VISIBLE_LINES = 5;
const MAX_TEXTAREA_HEIGHT = TEXTAREA_LINE_HEIGHT * MAX_VISIBLE_LINES + TEXTAREA_VERTICAL_PADDING;

export function ChatInput({ disabled }: { disabled?: boolean }) {
  const [value, setValue] = useState("");
  const [isResearchMode, setIsResearchMode] = useState(false);
  const [sourcePolicy, setSourcePolicy] = useState<ResearchSourcePolicy>("web");
  const [domainsText, setDomainsText] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const sendMessage = useChatStore((state) => state.sendMessage);
  const prepareResearch = useChatStore((state) => state.prepareResearch);
  const stopGeneration = useChatStore((state) => state.stopGeneration);
  const isGenerating = useChatStore((state) => state.isGenerating);

  const resizeTextarea = () => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "auto";
    const nextHeight = Math.min(textarea.scrollHeight, MAX_TEXTAREA_HEIGHT);
    textarea.style.height = `${nextHeight}px`;
    textarea.style.overflowY = textarea.scrollHeight > MAX_TEXTAREA_HEIGHT ? "auto" : "hidden";
  };

  useEffect(() => {
    resizeTextarea();
  }, [value]);

  const submit = () => {
    const content = value.trim();
    if (!content || disabled || isGenerating) return;
    setValue("");
    if (isResearchMode) {
      void prepareResearch(content, {
        sourcePolicy,
        domains: domainsText
          .split(/[\s,，]+/)
          .map((domain) => domain.trim())
          .filter(Boolean),
      });
      return;
    }
    void sendMessage(content);
  };

  return (
    <div className="shrink-0 bg-[var(--composer-fade)] px-3 pb-4 pt-2 sm:px-4">
      <div className="mx-auto max-w-3xl">
        <div className="overflow-hidden rounded-[28px] border border-[var(--composer-border)] bg-[var(--composer-bg)] shadow-[var(--composer-shadow)] focus-within:border-[var(--composer-focus)]">
          {isResearchMode ? (
            <div className="flex flex-wrap items-center gap-2 border-b border-[var(--border-subtle)] px-3 py-2">
              <div className="flex h-8 items-center gap-2 rounded-lg bg-[var(--surface-muted)] px-2 text-xs font-medium text-[var(--text-primary)]">
                <Microscope size={14} />
                Deep Research
              </div>
              <select
                className="h-8 rounded-lg bg-transparent px-2 text-xs text-[var(--text-secondary)] outline-none hover:bg-[var(--surface-hover)]"
                value={sourcePolicy}
                disabled={disabled || isGenerating}
                title="来源策略"
                onChange={(event) => setSourcePolicy(event.target.value as ResearchSourcePolicy)}
              >
                <option value="web">全网</option>
                <option value="includeDomains">仅指定站点</option>
                <option value="preferDomains">优先指定站点</option>
              </select>
              {sourcePolicy !== "web" ? (
                <input
                  className="h-8 min-w-[180px] flex-1 rounded-lg bg-[var(--surface-muted)] px-2 text-xs text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)]"
                  value={domainsText}
                  disabled={disabled || isGenerating}
                  placeholder="example.com, docs.example.com"
                  onChange={(event) => setDomainsText(event.target.value)}
                />
              ) : null}
            </div>
          ) : null}
          <div className="flex min-h-[58px] items-end gap-2 px-3 py-2">
            <button
              className={`mb-1 grid h-8 w-8 shrink-0 place-items-center rounded-full ${
                isResearchMode
                  ? "bg-[var(--button-primary)] text-[var(--button-primary-fg)]"
                  : "text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
              }`}
              title="Deep Research"
              disabled={disabled || isGenerating}
              onClick={() => setIsResearchMode((value) => !value)}
            >
              <Microscope size={16} />
            </button>
            <textarea
              ref={textareaRef}
              className="min-h-10 flex-1 resize-none overflow-hidden bg-transparent px-2 py-2 text-[15px] leading-6 text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)]"
              placeholder={disabled ? "请先新建会话" : isResearchMode ? "输入研究主题" : "输入消息"}
              rows={1}
              value={value}
              disabled={disabled}
              onChange={(event) => setValue(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  submit();
                }
              }}
            />
            {isGenerating ? (
              <button
                className="mb-1 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-[var(--button-primary)] text-[var(--button-primary-fg)] hover:bg-[var(--button-primary-hover)]"
                title="停止生成"
                onClick={() => void stopGeneration()}
              >
                <Square size={14} fill="currentColor" />
              </button>
            ) : (
              <button
                className="mb-1 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-[var(--button-primary)] text-[var(--button-primary-fg)] hover:bg-[var(--button-primary-hover)] disabled:bg-[var(--button-disabled)] disabled:text-[var(--button-disabled-fg)]"
                title={isResearchMode ? "生成研究计划" : "发送"}
                disabled={disabled || !value.trim()}
                onClick={submit}
              >
                <Send size={16} />
              </button>
            )}
          </div>
        </div>
        <p className="mt-2 text-center text-xs text-[var(--text-muted)]">
          {isResearchMode ? "Deep Research 会先生成计划，确认后开始执行。" : "AI 可能会出错，重要信息请核查。"}
        </p>
      </div>
    </div>
  );
}
