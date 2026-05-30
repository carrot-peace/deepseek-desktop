import { useEffect, useState } from "react";
import { KeyRound, X } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import type { AppSettings, ChatModel, ThinkingMode } from "../../lib/types";

export function SettingsDialog() {
  const isOpen = useSettingsStore((state) => state.isSettingsOpen);
  const setSettingsOpen = useSettingsStore((state) => state.setSettingsOpen);
  const settings = useSettingsStore((state) => state.settings);
  const saveSettings = useSettingsStore((state) => state.saveSettings);
  const saveDeepSeekApiKey = useSettingsStore((state) => state.saveDeepSeekApiKey);
  const saveTavilyApiKey = useSettingsStore((state) => state.saveTavilyApiKey);
  const deleteDeepSeekApiKey = useSettingsStore((state) => state.deleteDeepSeekApiKey);
  const deleteTavilyApiKey = useSettingsStore((state) => state.deleteTavilyApiKey);
  const hasDeepSeekApiKey = useSettingsStore((state) => state.hasDeepSeekApiKey);
  const hasTavilyApiKey = useSettingsStore((state) => state.hasTavilyApiKey);
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [deepseekKey, setDeepseekKey] = useState("");
  const [tavilyKey, setTavilyKey] = useState("");

  useEffect(() => setDraft(settings), [settings]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/35 px-4">
      <section className="w-full max-w-2xl overflow-hidden rounded-3xl border border-[var(--border-subtle)] bg-[var(--surface)] text-[var(--text-primary)] shadow-2xl">
        <header className="flex h-14 items-center justify-between border-b border-[var(--border-subtle)] px-5">
          <div className="flex items-center gap-2 text-sm font-semibold">
            <KeyRound size={18} />
            设置
          </div>
          <button className="grid h-8 w-8 place-items-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]" onClick={() => setSettingsOpen(false)}>
            <X size={18} />
          </button>
        </header>

        <div className="max-h-[70vh] space-y-5 overflow-y-auto px-5 py-5">
          <label className="block">
            <span className="text-sm font-medium">DeepSeek Base URL</span>
            <input
              className="mt-2 h-11 w-full cursor-not-allowed rounded-xl border border-[var(--border-subtle)] bg-[var(--surface-muted)] px-3 text-sm text-[var(--text-secondary)] outline-none"
              value={draft.deepseekBaseUrl}
              readOnly
            />
          </label>

          <div className="grid gap-4 sm:grid-cols-2">
            <label className="block">
              <span className="text-sm font-medium">默认模型</span>
              <select
                className="mt-2 h-11 w-full rounded-xl border border-[var(--border-subtle)] bg-[var(--control-bg)] px-3 text-sm outline-none focus:border-[var(--composer-focus)]"
                value={draft.defaultModel}
                onChange={(event) => setDraft({ ...draft, defaultModel: event.target.value as ChatModel })}
              >
                <option value="deepseek-v4-pro">deepseek-v4-pro</option>
                <option value="deepseek-v4-flash">deepseek-v4-flash</option>
              </select>
            </label>
            <label className="block">
              <span className="text-sm font-medium">默认推理模式</span>
              <select
                className="mt-2 h-11 w-full rounded-xl border border-[var(--border-subtle)] bg-[var(--control-bg)] px-3 text-sm outline-none focus:border-[var(--composer-focus)]"
                value={draft.defaultThinkingMode}
                onChange={(event) => setDraft({ ...draft, defaultThinkingMode: event.target.value as ThinkingMode })}
              >
                <option value="off">Normal</option>
                <option value="high">Reasoning</option>
                <option value="max">Deep Reasoning</option>
              </select>
            </label>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <label className="flex items-center gap-2 rounded-xl border border-[var(--border-subtle)] bg-[var(--control-bg)] px-3 py-3 text-sm">
              <input
                type="checkbox"
                checked={draft.defaultSearchEnabled}
                onChange={(event) => setDraft({ ...draft, defaultSearchEnabled: event.target.checked })}
              />
              默认开启联网搜索
            </label>
            <label className="flex items-center gap-2 rounded-xl border border-[var(--border-subtle)] bg-[var(--control-bg)] px-3 py-3 text-sm">
              <input
                type="checkbox"
                checked={draft.showReasoningContent}
                onChange={(event) => setDraft({ ...draft, showReasoningContent: event.target.checked })}
              />
              显示推理内容
            </label>
          </div>

          <SecretRow
            title="DeepSeek API Key"
            placeholder={hasDeepSeekApiKey ? "已保存，可输入新 Key 覆盖" : "sk-..."}
            value={deepseekKey}
            hasSecret={hasDeepSeekApiKey}
            onChange={setDeepseekKey}
            onSave={async () => {
              await saveDeepSeekApiKey(deepseekKey);
              setDeepseekKey("");
            }}
            onDelete={deleteDeepSeekApiKey}
          />
          <SecretRow
            title="Tavily API Key"
            placeholder={hasTavilyApiKey ? "已保存，可输入新 Key 覆盖" : "tvly-..."}
            value={tavilyKey}
            hasSecret={hasTavilyApiKey}
            onChange={setTavilyKey}
            onSave={async () => {
              await saveTavilyApiKey(tavilyKey);
              setTavilyKey("");
            }}
            onDelete={deleteTavilyApiKey}
          />
        </div>

        <footer className="flex justify-end gap-3 border-t border-[var(--border-subtle)] px-5 py-4">
          <button className="h-10 rounded-xl px-4 text-sm hover:bg-[var(--surface-hover)]" onClick={() => setSettingsOpen(false)}>
            取消
          </button>
          <button
            className="h-10 rounded-xl bg-[var(--button-primary)] px-4 text-sm font-medium text-[var(--button-primary-fg)] hover:bg-[var(--button-primary-hover)]"
            onClick={() => {
              void saveSettings(draft);
              setSettingsOpen(false);
            }}
          >
            保存设置
          </button>
        </footer>
      </section>
    </div>
  );
}

function SecretRow({
  title,
  placeholder,
  value,
  hasSecret,
  onChange,
  onSave,
  onDelete,
}: {
  title: string;
  placeholder: string;
  value: string;
  hasSecret: boolean;
  onChange: (value: string) => void;
  onSave: () => Promise<void>;
  onDelete: () => Promise<void>;
}) {
  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <span className="text-sm font-medium">{title}</span>
        <span className="text-xs text-[var(--text-secondary)]">{hasSecret ? "已配置" : "未配置"}</span>
      </div>
      <div className="flex gap-2">
        <input
          className="h-11 min-w-0 flex-1 rounded-xl border border-[var(--border-subtle)] bg-[var(--control-bg)] px-3 text-sm outline-none focus:border-[var(--composer-focus)]"
          type="password"
          value={value}
          placeholder={placeholder}
          onChange={(event) => onChange(event.target.value)}
        />
        <button
          className="h-11 rounded-xl bg-[var(--button-primary)] px-3 text-sm text-[var(--button-primary-fg)] hover:bg-[var(--button-primary-hover)] disabled:bg-[var(--button-disabled)] disabled:text-[var(--button-disabled-fg)]"
          disabled={!value.trim()}
          onClick={() => void onSave()}
        >
          保存
        </button>
        <button className="h-11 rounded-xl px-3 text-sm hover:bg-[var(--surface-hover)] hover:text-[var(--danger)]" onClick={() => void onDelete()}>
          删除
        </button>
      </div>
    </div>
  );
}
