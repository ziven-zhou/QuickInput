# QuickInput 项目知识库

> 本目录是 QuickInput 的项目知识入口，用于快速找到开发规则、代码结构、功能说明和故障排查资料。文档内容使用中文，技术名称、命令和代码标识保留原文。
>
> 整理日期：2026-11-12
>
> 导航链路：根目录 `AGENTS.md` → 本文件 → 规则、结构、功能和排障文档。

## 按目标阅读

| 我想做什么 | 建议阅读顺序 |
| --- | --- |
| 第一次参与开发 | [工程规范](rules/engineering.md) → [仓库结构](structure/repository.md) → [技术架构](structure/architecture.md) |
| 修改输入、确认或粘贴流程 | [快速输入](features/quick-input.md) → [技术架构](structure/architecture.md) → [macOS 自动粘贴排查](troubleshooting/macos-paste.md) |
| 修改内容库功能 | [内容库](features/content-library.md) → [数据模型](structure/data-model.md) |
| 修改设置、快捷键或托盘 | [设置与快捷键](features/settings-and-hotkeys.md) → [技术架构](structure/architecture.md) |
| 查看纯净输入与字体大小需求 | [纯净输入与字体大小需求](features/pure-input-mode.md) → [快速输入](features/quick-input.md) → [设置与快捷键](features/settings-and-hotkeys.md) |
| 构建、签名或安装应用 | [安全与发布](rules/security-release.md) → [macOS 自动粘贴排查](troubleshooting/macos-paste.md) |
| 了解产品范围和原始需求 | [产品设计](features/product-design.md) → [产品路线](features/product-roadmap.md) |
| 查阅历史自动粘贴问题 | [macOS 自动粘贴排查](troubleshooting/macos-paste.md) → [历史问题记录](troubleshooting/macos-paste-case.md) |
| 维护项目知识库 | [文档维护规则](rules/documentation.md) |

## 知识地图

### 规则

- [工程规范](rules/engineering.md)：代码风格、开发流程、测试与验证要求。
- [安全与发布](rules/security-release.md)：敏感数据、辅助功能权限、生产构建与安装约束。
- [文档维护规则](rules/documentation.md)：知识库目录边界、内容来源和更新方式。

### 结构

- [仓库结构](structure/repository.md)：根目录、前端、Rust 后端和生成目录的职责。
- [技术架构](structure/architecture.md)：Tauri 进程边界、窗口、命令、事件、快捷键和启动流程。
- [数据模型](structure/data-model.md)：`Settings`、`ContentEntry` 和本地持久化格式。

### 功能

- [快速输入](features/quick-input.md)：唤起、编辑、两种内容模式、确认、取消、可选粘贴并发送快捷键和粘贴闭环。
- [内容库](features/content-library.md)：手动新增、编辑、搜索、删除和直接粘贴。
- [设置与快捷键](features/settings-and-hotkeys.md)：设置项、快捷键录制与冲突校验、托盘和单实例行为。
- [纯净输入模式与输入框字体大小](features/pure-input-mode.md)：已实现功能的交互、字号设置、影响范围和验收标准。
- [产品设计](features/product-design.md)：原始产品需求、核心流程、UI 设计、MVP 规划和 v0.2 需求。
- [产品路线](features/product-roadmap.md)：原始设计范围、当前实现状态和后续方向。

### 排障

- [macOS 自动粘贴排查](troubleshooting/macos-paste.md)：辅助功能权限、应用签名身份、重建安装和诊断日志。
- [macOS 自动粘贴历史记录](troubleshooting/macos-paste-case.md)：原始问题现象、根因、修复结论和验证命令。

## 文档来源

| 来源 | 整理位置 |
| --- | --- |
| 根目录 `AGENTS.md` | [工程规范](rules/engineering.md)、[安全与发布](rules/security-release.md)、[仓库结构](structure/repository.md) |
| 原 `DESIGN.md` | [产品设计](features/product-design.md)、[产品路线](features/product-roadmap.md)、[快速输入](features/quick-input.md)、[内容库](features/content-library.md)、[设置与快捷键](features/settings-and-hotkeys.md) |
| 原 `PROBLEM_PASTE.md` | [macOS 自动粘贴历史记录](troubleshooting/macos-paste-case.md)、[macOS 自动粘贴排查](troubleshooting/macos-paste.md)、[技术架构](structure/architecture.md) |
| `src/`、`src-tauri/` 源码 | [仓库结构](structure/repository.md)、[技术架构](structure/architecture.md)、[数据模型](structure/data-model.md) |

原根目录中的产品设计和问题记录已迁移到 `.codex`，根目录不再保留这两个文件。`.codex` 是项目知识的唯一内容目录，根目录 `AGENTS.md` 只负责进入本 MOC。

## 快速事实

- 应用名称：QuickInput。
- 技术栈：Tauri 2、Rust、Vite、原生 HTML/CSS/JavaScript。
- 三个窗口：`main`、`settings`、`content`。
- 默认唤起快捷键：`Alt+Space`。
- 默认确认快捷键：`Alt+Enter`。
- 默认取消快捷键：`Escape`。
- 可选粘贴并发送快捷键：`Shift+Enter`，默认关闭；原确认快捷键仍仅粘贴。
- 本地配置：Tauri 应用配置目录下的 `settings.json`。
- 本地内容库：Tauri 应用配置目录下的 `content.json`。
- 发布标识：`com.quickinput.quickinput`。

## 内容边界

- `features/product-design.md` 是产品设计和迭代目标的来源。
- `features/product-roadmap.md` 以当前代码为准记录实现状态。
- `troubleshooting/macos-paste-case.md` 是 macOS 自动粘贴问题的历史记录。
- `troubleshooting/macos-paste.md` 是当前排障流程，出现差异时以当前代码和实测结果为准。
- `.codex` 功能文档以当前代码行为为准，并明确标注与设计目标不一致的地方。
- 修改行为时，应同步修改对应功能文档；涉及命令、事件、配置字段或持久化格式时，还应更新结构文档。
