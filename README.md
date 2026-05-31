# DeepSeek Desktop

基于 Tauri 2 的 DeepSeek 桌面客户端，支持多轮对话、深度推理和联网搜索。

## 技术栈

| 层 | 技术 |
|---|------|
| 框架 | Tauri 2 |
| 前端 | React 19 + TypeScript + Tailwind CSS |
| 状态管理 | Zustand |
| 后端 | Rust (reqwest / rusqlite / keyring) |
| 构建 | Vite + tauri-cli |

## 功能特性

- 多轮对话，流式输出
- 三种推理模式：Normal / Reasoning / Deep Reasoning
- DeepSeek 联网搜索（基于 Tavily）
- Deep Research 深度研究任务
- API Key 通过系统钥匙串安全存储
- 本地 SQLite 持久化对话记录

## 开发

```bash
# 安装前端依赖
npm install

# 启动开发模式
npm run tauri dev

# 构建生产包
npm run tauri build
```

## 项目结构

```
├── src/                    # 前端 React 源码
│   ├── components/         # UI 组件
│   ├── stores/             # Zustand 状态管理
│   └── lib/                # 类型定义与 Tauri API 封装
├── src-tauri/              # Rust 后端
│   ├── src/                # Rust 源码
│   ├── migrations/         # 数据库迁移
│   └── icons/              # 应用图标
├── package.json
└── vite.config.ts
```
