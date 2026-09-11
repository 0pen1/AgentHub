# AgentHub

一个用于启动和管理 AI 编码 Agent 会话的桌面应用（Tauri 2 + React + TypeScript）。支持多家 Agent CLI，内置技能库 / MCP / 提示词库的托管，以及「专家」预设一键启动会话。

## 功能

- **多 Agent 支持** — Claude Code、Codex、Gemini CLI、OpenCode、pi 一键启动，各适配器负责命令拼装与状态识别
- **会话级配置隔离** — 所选技能 / MCP / 提示词注入到会话私有的临时目录（环境变量重定向 + CLI 参数），不写入项目目录，不改全局配置；删除会话即清理
- **对话续传** — Claude 会话钉住固定 session id，重启应用后点击历史会话自动 `--resume`，接着上次的对话继续
- **PTY 会话管理** — 内嵌终端（xterm.js + PTY），切页/重启不丢输出，滚动缓冲快照恢复；会话恢复时显示启动状态
- **技能库（Skills）** — 托管于 `~/.agenthub/library`，支持 frontmatter 元数据与标签，启动时按会话注入（Unix symlink / 其他平台拷贝）
- **MCP 服务器** — 托管 JSON 配置，一键注入到支持 MCP 的 Agent
- **提示词库（Instructions）** — Markdown 提示词托管与勾选注入
- **专家预设（Experts）** — 将 agent + 技能 + MCP + 提示词组合存为预设，卡片墙一键启动
- **卡片墙配置管理** — 技能/MCP/提示词/专家四个标签页，搜索 + 标签过滤 + 本机扫描一键导入
- **会话列表** — 历史会话恢复、状态徽章、活动排序、优雅退出（SIGTERM → SIGKILL）

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2（Rust 后端） |
| 前端 | React 19 + TypeScript + Vite |
| 终端 | xterm.js + @xterm/addon-fit + webgl |
| 状态 | SQLite（`presets` / `sessions` 等） |
| 样式 | CSS 变量主题（原生窗口风格） |

## 开发

```bash
npm install
npx tauri dev
```

Rust 单元测试（配置注入、适配器、滚动缓冲等）：

```bash
cd src-tauri && cargo test
```

## 构建

```bash
npm run build        # 前端构建
npx tauri build      # 打包 .app / .dmg
```

## 发布

push 一个 `v*` tag（如 `v0.1.0`）会触发 GitHub Actions（[.github/workflows/release.yml](.github/workflows/release.yml)），自动构建 macOS（Apple Silicon / Intel）、Linux、Windows 四个目标的安装包并发布到 GitHub Releases。版本号以 `src-tauri/tauri.conf.json` 为准。

## 各 Agent 的注入方式

| Agent | MCP 注入 | 提示词注入 | 技能注入 | 认证 |
|---|---|---|---|---|
| Claude Code | `--strict-mcp-config` + 会话 `mcp.json` | `CLAUDE.md` 标记块替换（可重复注入） | 会话插件目录 `--plugin-dir` | 继承 `~/.claude` |
| Codex | 会话 `config.toml` 结构化合并（`CODEX_HOME`） | 会话 `AGENTS.md` | 会话 `skills/` | `auth.json` 带入会话 |
| Gemini CLI | `GEMINI_CLI_SYSTEM_SETTINGS_PATH` 会话设置 | 会话文件 + `context.fileName` | — | 继承 `~/.gemini` |
| pi | — | — | `PI_CODING_AGENT_DIR` + settings `skills` 数组 | 继承 `~/.pi` |
| OpenCode | `OPENCODE_CONFIG` 会话配置 | 会话 `instructions.md` | — | 继承全局配置 |

## 项目结构

```
agenthub/
├── src/                  # React 前端
│   ├── pages/            # Launcher / Sessions / ConfigLib
│   ├── components/       # 终端、选择器、卡片墙组件
│   └── lib/              # Tauri API 封装与类型
└── src-tauri/            # Rust 后端
    └── src/
        ├── agent/        # Agent 静态定义表 + 各 CLI 适配器
        ├── pty/          # PTY 进程管理、滚动缓冲、优雅退出
        ├── session/      # 会话存储（SQLite）
        └── config/       # 技能/MCP/提示词/专家库
```

## 数据目录

- 应用数据：`~/Library/Application Support/com.agenthub.app/`（SQLite）
- 托管库：`~/.agenthub/library/`
- 会话临时配置：`~/.agenthub/sessions/<id>/`（删除会话时清理）
- 终端快照：`~/.agenthub/scrollback/`（启动时自动清理孤儿快照）

## License

MIT
