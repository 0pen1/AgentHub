# CLAUDE.md

AgentHub — 多 Agent CLI 会话启动器（Tauri 2 + React 19 + TypeScript + Rust）。

## 常用命令

```bash
npx tauri dev              # 开发运行（前端 + Rust 热重载）
npx tauri build            # 打包 .app / .dmg
npm run build              # 仅前端构建（含 tsc 类型检查）
cd src-tauri && cargo test # Rust 单元测试（33 个）
cd src-tauri && cargo check
```

前端无测试；Rust 测试覆盖注入层（adapters、instruction 合并、滚动缓冲）。

## 架构速览

```
src/                    React 前端（三页面 keep-alive 常驻，只切 display）
  pages/Launcher.tsx    四步向导：专家/Agent → 目录 → 能力 → 启动
  pages/Sessions.tsx    会话树 + 内嵌终端
  pages/ConfigLib.tsx   卡片墙：专家/Skills/MCP/提示词（1400 行，拆分待做）
src-tauri/src/
  agent/registry.rs     AgentRegistry — 安装探测（60s TTL 缓存）
  agent/adapters/       KNOWN_AGENTS 静态定义表（单一事实来源）+ 各 CLI 适配器
  config/injector.rs    ConfigInjector — 启动前的配置注入编排
  pty/                  portable-pty 管理、滚动缓冲（512KB 尾部）、优雅退出
  session/store.rs      SQLite（sessions + presets）
  commands.rs           Tauri 命令层
```

## 关键设计约定（改动前必读）

**1. 会话级配置隔离是核心承诺。** 任何适配器都不得向项目目录或 agent 全局
配置写入任何内容。注入一律走会话私有目录 `~/.agenthub/sessions/<id>/` +
环境变量重定向（`CODEX_HOME` / `GEMINI_CLI_SYSTEM_SETTINGS_PATH` /
`PI_CODING_AGENT_DIR` / `OPENCODE_CONFIG`）或 CLI 参数（claude 的
`--strict-mcp-config` / `--plugin-dir` / `--setting-sources`）。新增适配器
时先查该 CLI 是否有等价的 env/参数级隔离机制，没有才考虑会话目录内的变通，
绝不能像旧版 gemini 适配器那样直接写 `<work_dir>/.gemini/` 或覆盖 GEMINI.md。

**2. 双表必须同步。** `adapters/mod.rs` 的 `KNOWN_AGENTS` 与 `get_adapter()`
是同一个模块，新增 Agent 两处都要加；测试
`every_known_agent_has_an_adapter` 会拦截漏项。registry 只消费 KNOWN_AGENTS。

**3. 会话状态机的诚实性。** `launch_session`/`restart_session` 会先写 DB
status=running，真正 spawn 在 `pty_attach`。spawn 失败必须回写 exited 并
广播 `sessions-changed`（已实现，勿破坏）。改状态用
`update_status_if_running` 保住 killed，不要用裸 `update_status`。

**4. 优雅退出有固定顺序。** `graceful_kill`：writer flush（EOF）→ SIGTERM
+ 3s 宽限 → SIGKILL。`ChildKiller::kill` 在 Unix 上就是 SIGKILL，只能作
最后手段。

**5. 前端 PTY 管线在模块级，不在组件里。** `Sessions.tsx` 顶部模块级
`buffers`/`writeFns`/`unlisteners` Map 保证事件监听器每个会话恰好注册一次
（历史上组件级注册泄漏导致输出重影）。`notifyClosed`/`notifyFirstOutput`
是页面挂载时设置的模块级回调。

**6. Claude 注入的三个坑**（`adapters/claude.rs`）：
- `--setting-sources project,local` 排除全局 skills，但要手动回注全局
  settings 的 `env` 和 `enabledPlugins`，否则用户认证/插件丢失
- 提示词用标记块（`agenthub:session-instructions:*`）替换式写入 CLAUDE.md，
  可重复注入不重复
- resume 探测：`~/.claude/projects/<munged-work_dir>/<session-id>.jsonl`，
  munge 规则 = 非字母数字全替换 `-`（已用 590 个真实目录验证过）

## 代码风格

- Rust：与周围代码一致，注释解释「为什么」而非「是什么」；关键机制注释用
  英文，用户可见文案（错误信息）用中文
- 前端：Tailwind 类 + CSS 变量（`var(--bg-primary)` 等），不用硬编码颜色
  （终端主题除外）；组件内联 `style` 与 Tailwind 混用是既有惯例
- 提交信息：中文，`fix:`/`feat:`/`docs:`/`refactor:` 前缀，正文说明动机
  和关键决策；不提交未验证的改动（build + cargo test 必须先过）

## 平台注意

- symlink 注入一律用 `adapters::link_or_copy`（Unix symlink / 其他平台递归
  拷贝），禁止裸 `#[cfg(unix)]` 静默跳过
- PATH 修复在 `lib.rs::run()` 按平台分支（macOS zsh source / Linux login
  shell 合并 / Windows 追加工具目录），新平台支持时记得补
- 版本号单一来源：`src-tauri/tauri.conf.json`（package.json / Cargo.toml
  保持一致，发版时三处同改）
