import { useEffect, useRef, useState } from "react";
import { Send, Square } from "lucide-react";
import { useChatStore } from "../../stores/chatStore";

const TEXTAREA_LINE_HEIGHT = 24;
const TEXTAREA_VERTICAL_PADDING = 16;
const MAX_VISIBLE_LINES = 5;
const MAX_TEXTAREA_HEIGHT = TEXTAREA_LINE_HEIGHT * MAX_VISIBLE_LINES + TEXTAREA_VERTICAL_PADDING;

export function ChatInput({ disabled }: { disabled?: boolean }) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const sendMessage = useChatStore((state) => state.sendMessage);
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
    void sendMessage(content);
  };

  return (
    <div className="shrink-0 bg-[var(--composer-fade)] px-3 pb-4 pt-2 sm:px-4">
      <div className="mx-auto max-w-3xl">
        <div className="flex min-h-[58px] items-end gap-2 rounded-[28px] border border-[var(--composer-border)] bg-[var(--composer-bg)] px-3 py-2 shadow-[var(--composer-shadow)] focus-within:border-[var(--composer-focus)]">
          <textarea
            ref={textareaRef}
            className="min-h-10 flex-1 resize-none overflow-hidden bg-transparent px-2 py-2 text-[15px] leading-6 text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)]"
            placeholder={disabled ? "请先新建会话" : "输入消息"}
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
              title="发送"
              disabled={disabled || !value.trim()}
              onClick={submit}
            >
              <Send size={16} />
            </button>
          )}
        </div>
        <p className="mt-2 text-center text-xs text-[var(--text-muted)]">AI 可能会出错，重要信息请核查。</p>
      </div>
    </div>
  );
}
