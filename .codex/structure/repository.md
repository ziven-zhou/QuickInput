# 仓库结构

## 总览

QuickInput 是 Tauri 2 桌面应用。前端位于 `src/`，Rust 后端位于 `src-tauri/`，构建后的前端位于 `dist/`。

```text
InputDialog/
├── .codex/                 项目知识库
│   ├── MOC.md              项目知识总入口
│   ├── rules/              工程规则、安全和文档规范
│   ├── structure/          仓库、架构和数据模型
│   ├── features/           产品设计、功能和路线
│   └── troubleshooting/    排查流程和历史问题
├── AGENTS.md               仓库级约束与知识库导航
├── src/                    前端页面、交互逻辑和样式
├── src-tauri/              Rust 后端和 Tauri 配置
├── dist/                   Vite 构建产物
├── node_modules/           前端依赖
├── package.json            前端依赖与脚本
├── package-lock.json       前端依赖锁定文件
├── vite.config.js          Vite 多页面构建配置
└── run.sh                  开发模式快捷启动脚本
```

## 根目录文件

| 文件 | 职责 |
| --- | --- |
| `AGENTS.md` | 仓库级约束和 `.codex/MOC.md` 导航入口 |
| `.gitignore` | 排除依赖、构建产物、生成 schema、本地编辑器配置、日志和本地环境文件；保留环境示例文件 |
| `package.json` | 前端项目信息与 `dev`、`build`、`tauri` 脚本 |
| `vite.config.js` | 以 `src/` 为根目录，构建 `index`、`settings`、`content` 三个入口 |
| `run.sh` | 进入项目目录并执行 `npm run tauri dev` |

## 项目知识库

| 路径 | 职责 |
| --- | --- |
| `.codex/MOC.md` | 所有项目知识的总入口 |
| `.codex/rules/` | 工程规范、安全发布、文档维护 |
| `.codex/structure/` | 仓库结构、技术架构、数据模型 |
| `.codex/features/` | 产品设计、产品路线、输入、内容库、设置等功能说明 |
| `.codex/troubleshooting/` | 当前排查流程和历史问题记录 |

`.codex/.obsidian/` 是本地 Markdown 编辑器的配置，不属于项目业务文档。

## 前端

| 文件 | 职责 |
| --- | --- |
| `src/index.html` | 主输入浮层结构 |
| `src/settings.html` | 设置窗口结构 |
| `src/content.html` | 内容库窗口、编辑弹窗和预览结构 |
| `src/main.js` | 主输入窗口交互、确认、取消、Markdown 预览和事件监听 |
| `src/settings.js` | 设置加载、快捷键录制、表单保存和状态提示 |
| `src/content.js` | 内容库加载、搜索、新增、编辑、删除和直接粘贴 |
| `src/confirmation-shortcuts.js` | 确认动作快捷键判定与分阶段错误提示 |
| `src/confirmation-shortcuts.test.js` | 粘贴/发送选择、输入法与重复按键保护、错误提示的单元测试 |
| `src/input-preferences.js` | 纯净模式和输入框字号的共享常量、校验与应用逻辑 |
| `src/input-preferences.test.js` | 输入偏好逻辑的前端单元测试 |
| `src/page-shortcuts.js` | 所有页面共享的快捷键解析和页面内快捷键切换 |
| `src/markdown.js` | 前端 Markdown 渲染，供 Markdown 预览使用 |
| `src/styles.css` | 三个窗口共享的 UI 样式 |

前端构建入口由 `vite.config.js` 的 `rollupOptions.input` 指定，输出目录为 `../dist`。

## Rust 后端

| 文件 | 职责 |
| --- | --- |
| `src-tauri/src/main.rs` | 桌面程序入口，调用 `quickinput_lib::run()` |
| `src-tauri/src/lib.rs` | 核心状态、Tauri 命令、内容库持久化、快捷键、托盘、窗口和 Markdown/剪贴板逻辑 |
| `src-tauri/src/platform.rs` | macOS、Windows、Linux 的前台进程识别、应用激活、按键释放检查和粘贴/回车模拟 |
| `src-tauri/src/confirmation.rs` | 确认动作、共享粘贴锁、焦点保护、粘贴/回车时序和模拟输入单元测试 |

核心状态 `AppState` 维护：

- 当前 `Settings`
- 唤起输入框前的前台应用 PID
- 已注册的全局快捷键
- 内存中的内容库条目
- 主输入框与内容库共享的粘贴操作锁 `paste_in_progress`

详细调用关系见[技术架构](architecture.md)。

## Tauri 配置

| 文件 | 职责 |
| --- | --- |
| `src-tauri/tauri.conf.json` | 应用名称、版本、bundle identifier、窗口、构建命令和打包信息 |
| `src-tauri/Cargo.toml` | Rust 包、Tauri 插件和平台依赖 |
| `src-tauri/capabilities/default.json` | `main`、`settings`、`content` 三个窗口的权限 |
| `src-tauri/Info.plist` | macOS `LSUIElement` 配置 |
| `src-tauri/entitlements.plist` | macOS entitlements |
| `src-tauri/build.rs` | Tauri 构建脚本 |
| `src-tauri/icons/` | 应用图标 |
| `src-tauri/gen/schemas/` | Tauri 生成的 schema |

## 生成和依赖目录

以下目录通常由工具生成，不应手工修改：

- `dist/`：Vite 构建结果。
- `node_modules/`：npm 依赖。
- `src-tauri/target/`：Cargo 构建结果；存在时不要作为源码编辑。
- `src-tauri/gen/schemas/`：Tauri schema 生成文件。

`src-tauri/icons/` 是项目资源，不应按照普通生成文件处理。

根目录 `.gitignore` 已排除上述生成和依赖目录。源码、测试、图标、`package-lock.json`、`src-tauri/Cargo.lock` 及 `.codex` 项目文档应纳入版本管理；`.codex/.obsidian/` 仅为本地编辑器配置，已忽略。忽略规则不会删除磁盘上的文件，也不会自动取消已经跟踪的文件。

## 主要入口

- 前端主入口页面：`src/index.html`
- 前端主交互脚本：`src/main.js`
- Rust 库入口：`src-tauri/src/lib.rs` 中的 `run()`
- 桌面二进制入口：`src-tauri/src/main.rs`
- Tauri 应用配置：`src-tauri/tauri.conf.json`

## 目录维护建议

- 新增页面时，同时更新 Vite 多入口、Tauri 窗口配置、capability 和 `.codex` 结构文档。
- 新增共享前端逻辑时，优先放入独立模块，不继续扩大页面脚本。
- 平台相关代码集中放在 `platform.rs`，不要让平台分支散落到页面逻辑中。
- 新增持久化字段时更新[数据模型](data-model.md)。
