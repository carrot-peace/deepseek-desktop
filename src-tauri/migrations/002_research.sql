CREATE TABLE IF NOT EXISTS research_tasks (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  user_message_id TEXT NOT NULL,
  assistant_message_id TEXT,
  topic TEXT NOT NULL,
  status TEXT NOT NULL,
  source_policy TEXT NOT NULL,
  domains_json TEXT NOT NULL DEFAULT '[]',
  plan_json TEXT NOT NULL,
  report TEXT NOT NULL DEFAULT '',
  error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
  FOREIGN KEY (user_message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS research_sources (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  source_number INTEGER NOT NULL,
  title TEXT NOT NULL,
  url TEXT NOT NULL,
  snippet TEXT NOT NULL,
  published_at TEXT,
  source_domain TEXT,
  raw_content TEXT,
  score REAL,
  source_query TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES research_tasks(id) ON DELETE CASCADE,
  UNIQUE(task_id, url),
  UNIQUE(task_id, source_number)
);

CREATE TABLE IF NOT EXISTS research_activities (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  activity_type TEXT NOT NULL,
  title TEXT NOT NULL,
  detail TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES research_tasks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_research_tasks_conversation_updated
  ON research_tasks(conversation_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_research_sources_task_number
  ON research_sources(task_id, source_number ASC);

CREATE INDEX IF NOT EXISTS idx_research_activities_task_created
  ON research_activities(task_id, created_at ASC);
