import { useState } from "react";
import { ChatView } from "../chat/ChatView";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

export function AppLayout() {
  const [isSidebarPinned, setSidebarPinned] = useState(true);
  const [isMobileSidebarOpen, setMobileSidebarOpen] = useState(false);

  return (
    <main className="relative flex h-full overflow-hidden bg-[var(--app-bg)] text-[var(--text-primary)]">
      <Sidebar
        isPinned={isSidebarPinned}
        isMobileOpen={isMobileSidebarOpen}
        onCloseMobile={() => setMobileSidebarOpen(false)}
      />

      {isMobileSidebarOpen ? (
        <button
          className="fixed inset-0 z-30 bg-black/40 sm:hidden"
          aria-label="Close sidebar"
          onClick={() => setMobileSidebarOpen(false)}
        />
      ) : null}

      <section className="flex min-w-0 flex-1 flex-col bg-[var(--main-bg)]">
        <TopBar
          isSidebarPinned={isSidebarPinned}
          onToggleSidebar={() => setSidebarPinned((value) => !value)}
          onOpenMobileSidebar={() => setMobileSidebarOpen(true)}
        />
        <ChatView />
      </section>
    </main>
  );
}
