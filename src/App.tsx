import { useEffect } from "react";
import { AppLayout } from "./components/layout/AppLayout";
import { SettingsDialog } from "./components/settings/SettingsDialog";
import { useChatStore } from "./stores/chatStore";
import { useSettingsStore } from "./stores/settingsStore";

export default function App() {
  const loadConversations = useChatStore((state) => state.loadConversations);
  const initializeEventListeners = useChatStore((state) => state.initializeEventListeners);
  const loadSettings = useSettingsStore((state) => state.loadSettings);

  useEffect(() => {
    void initializeEventListeners();
    void loadSettings();
    void loadConversations();
  }, [initializeEventListeners, loadConversations, loadSettings]);

  return (
    <>
      <AppLayout />
      <SettingsDialog />
    </>
  );
}
