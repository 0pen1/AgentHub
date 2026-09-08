# AgentHub

一个用于启动和管理 AI 编码 Agent 会话的 macOS 桌面应用（Tauri 2 + React + TypeScript）。支持多家 Agent CLI，内置技能库 / MCP / 提示词库的托管，以及「专家」预设一键启动会话。

## 功能

- **多 Agent 支持** — Claude Code、Codex、Gemini CLI、OpenCode、pi 一键启动，各适配器负责命令拼装与状态识别
- **PTY 会话管理** — 内嵌终端（xterm.js + PTY），切页/重启不丢输出，滚动缓冲快照恢复
- **技能库（Skills）** — 托管于 `~/.agenthub/library`，支持 frontmatter 元数据与标签，启动时按会话注入（symlink）
- **MCP 服务器** — 托管 JSON 配置，一键注入到支持 MCP 的 Agent
- **提示词库（Instructions）** — Markdown 提示词托管与勾选注入
- **专家预设（Experts）** — 将 agent + 技能 + MCP + 提示词组合存为预设，卡片墙一键启动
- **卡片墙配置管理** — 技能/MCP/提示词/专家四个标签页，搜索 + 标签过滤 + 本机扫描一键导入
- **会话列表** — 历史会话恢复、状态徽章、活动排序

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2（Rust 后端） |
| 前端 | React 18 + TypeScript + Vite |
| 终端 | xterm.js + @xterm/addon-fit + webgl |
| 状态 | SQLite（`presets` / `sessions` 等） |
| 样式 | CSS 变量主题（原生窗口风格） |

## 开发

```bash
npm install
npx tauri dev
```

## 构建

```bash
npm run build        # 前端构建
npx tauri build      # 打包 .app / .dmg
```

## 项目结构

```
agenthub/
├── src/                  # React 前端
│   ├── pages/            # Launcher / Sessions / ConfigLib
│   ├── components/       # 终端、选择器、卡片墙组件
│   └── lib/              # Tauri API 封装与类型
└── src-tauri/            # Rust 后端
    └── src/
        ├── agent/        # 各 Agent CLI 适配器
        ├── pty/          # PTY 进程管理、滚动缓冲
        ├── session/      # 会话存储（SQLite）
        └── config/       # 技能/MCP/提示词/专家库
```

## 数据目录

- 应用数据：`~/Library/Application Support/com.agenthub.app/`（SQLite）
- 托管库：`~/.agenthub/library/`

## License

MIT
