import { create } from "zustand";
import { commands } from "../lib/tauri";
import type { AppSettings } from "../lib/types";

const fallbackSettings: AppSettings = {
  deepseekBaseUrl: "https://api.deepseek.com",
  defaultModel: "deepseek-v4-pro",
  defaultThinkingMode: "off",
  defaultSearchEnabled: false,
  showReasoningContent: false,
  searchProvider: "tavily",
};

interface SettingsState {
  settings: AppSettings;
  hasDeepSeekApiKey: boolean;
  hasTavilyApiKey: boolean;
  isSettingsOpen: boolean;
  error?: string;
  loadSettings: () => Promise<void>;
  saveSettings: (settings: AppSettings) => Promise<void>;
  saveDeepSeekApiKey: (key: string) => Promise<void>;
  saveTavilyApiKey: (key: string) => Promise<void>;
  deleteDeepSeekApiKey: () => Promise<void>;
  deleteTavilyApiKey: () => Promise<void>;
  setSettingsOpen: (open: boolean) => void;
  setError: (error?: string) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: fallbackSettings,
  hasDeepSeekApiKey: false,
  hasTavilyApiKey: false,
  isSettingsOpen: false,
  async loadSettings() {
    try {
      const [settings, hasDeepSeekApiKey, hasTavilyApiKey] = await Promise.all([
        commands.getSettings(),
        commands.hasSecret("deepseek_api_key"),
        commands.hasSecret("tavily_api_key"),
      ]);
      set({ settings, hasDeepSeekApiKey, hasTavilyApiKey, error: undefined });
    } catch (error) {
      set({ error: String(error) });
    }
  },
  async saveSettings(settings) {
    await commands.saveSettings(settings);
    set({ settings, error: undefined });
  },
  async saveDeepSeekApiKey(key) {
    await commands.setSecret("deepseek_api_key", key.trim());
    set({ hasDeepSeekApiKey: true, error: undefined });
  },
  async saveTavilyApiKey(key) {
    await commands.setSecret("tavily_api_key", key.trim());
    set({ hasTavilyApiKey: true, error: undefined });
  },
  async deleteDeepSeekApiKey() {
    await commands.deleteSecret("deepseek_api_key");
    set({ hasDeepSeekApiKey: false, error: undefined });
  },
  async deleteTavilyApiKey() {
    await commands.deleteSecret("tavily_api_key");
    set({ hasTavilyApiKey: false, error: undefined });
  },
  setSettingsOpen(open) {
    set({ isSettingsOpen: open });
  },
  setError(error) {
    set({ error });
  },
}));
