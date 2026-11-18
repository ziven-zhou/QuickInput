# 工程规范

## 适用范围

本文适用于 QuickInput 整个仓库。项目知识统一通过根目录 `AGENTS.md` 进入 [MOC](../MOC.md)，再按 MOC 读取具体文档。若 `AGENTS.md` 或用户的新指令与本文冲突，以更新、更明确的指令为准。

## 代码探索

- 仓库根目录存在 `.codegraph/` 时，优先使用 CodeGraph 理解或定位代码。
- 优先使用 MCP 工具 `codegraph_explore`；不可用时使用 `codegraph explore "<问题或符号>"`。
- 只有在没有 CodeGraph 覆盖时，才使用 `rg`、`find` 等文本搜索方式。
- 当前仓库未发现 `.codegraph/` 时，可直接使用常规搜索和文件阅读。

## 代码风格

- HTML、CSS、JavaScript 使用两个空格缩进。
- Rust 使用 `rustfmt` 默认格式。
- Rust 命名使用 `snake_case`。
- JavaScript 命名使用 `camelCase`。
- UI 文案优先使用中文。
- 代码注释使用英文。
- UI 页面保持小而聚焦，新增逻辑优先放入职责明确的模块。
- 快捷键默认值必须通过 Rust `Settings` 和 `src/settings.js` 统一管理，不在业务代码中硬编码新的默认值。

## 模块边界

- `src/`：渲染层页面、交互逻辑和共享样式。
- `src-tauri/src/lib.rs`：Tauri 命令、状态、持久化、窗口、托盘和快捷键。
- `src-tauri/src/platform.rs`：平台相关的焦点切换和粘贴模拟。
- `src-tauri/tauri.conf.json`：窗口、打包和开发服务器配置。
- `.codex/features/product-design.md`：产品设计和需求来源。

更完整的文件职责见[仓库结构](../structure/repository.md)。

## 开发流程

1. 先阅读相关文档和现有实现，确认行为边界。
2. 复杂功能先形成简短实施计划。
3. 新功能或缺陷修复在可行时优先采用测试驱动方式。
4. 输入、持久化和跨平台操作要考虑失败路径。
5. 修改后运行与风险相符的构建和检查命令。
6. 行为发生变化时，同步更新 `.codex` 对应文档。

## 测试与验证

当前已有输入偏好逻辑的前端单元测试。最低验证要求：

```bash
npm test
npm run build
cd src-tauri && cargo check
```

其他适用命令：

```bash
npm run dev
npm run tauri dev
npm run tauri build
./run.sh
```

测试建议：

- 内容迁移、快捷键解析等共享 Rust 逻辑应增加 `#[cfg(test)]` 单元测试。
- 输入偏好、字号校验等共享前端逻辑应增加 `node:test` 单元测试。
- 后续前端集成测试应覆盖通过 Tauri 命令完成的用户主流程。
- 涉及 macOS 粘贴、权限、窗口焦点或托盘行为时，需要补充实际应用中的手动验证。
- 修改安装版本前，必须重新构建并替换 `/Applications/QuickInput.app`，仅运行 `npm run build` 不会更新已安装应用。

## 输入与持久化

- 持久化 `content.json` 前必须校验空内容、格式和重复内容。
- 内容库只保存用户主动添加的内容，不得把自动记录带入新存储。
- 用户主动保存的敏感内容仍会落到本地文件，不能把内容库视为敏感信息保护机制。

## 提交规范

- 使用祈使式摘要，例如 `feat: add content library`、`fix: persist window position`。
- 首行控制在 72 个字符以内。
- 非直观改动应在提交正文说明原因。
- Pull Request 应说明问题、方案和手动验证步骤。
- UI 改动应附截图，并关联相关 Issue。

## 相关文档

- [安全与发布](security-release.md)
- [文档维护规则](documentation.md)
- [仓库结构](../structure/repository.md)
