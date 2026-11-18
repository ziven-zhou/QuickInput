# 技术架构

## 架构概览

```text
┌──────────────────────────────── Frontend ────────────────────────────────┐
│ main.js / settings.js / content.js                                      │
│ page-shortcuts.js / markdown.js / styles.css                            │
└───────────────────────────────┬──────────────────────────────────────────┘
                                │ invoke / listen / emit
┌───────────────────────────────▼──────────────────────────────────────────┐
│ Tauri Core: src-tauri/src/lib.rs                                         │
│ Commands / AppState / Windows / Tray / Global Shortcuts / Persistence    │
└───────────────┬───────────────────────────────┬──────────────────────────┘
                │                               │
┌───────────────▼──────────────┐  ┌─────────────▼──────────────────────────┐
│ platform.rs / confirmation.rs│  │ Local storage / clipboard              │
│ focus + paste / enter        │  │ settings.json / content.json / arboard │
└──────────────────────────────┘  └────────────────────────────────────────┘
```

前端通过 `@tauri-apps/api` 的 `invoke` 调用 Rust 命令，通过 `listen` 接收后端事件。Rust 使用 `AppState` 管理共享状态。

## 窗口

| 标签 | 页面 | 默认尺寸 | 关键配置 |
| --- | --- | --- | --- |
| `main` | `src/index.html` | 720 × 240 | 无边框、透明、置顶、隐藏启动、可调整大小、跳过任务栏 |
| `settings` | `src/settings.html` | 680 × 620 | 双栏自动保存；无边框、透明、置顶、隐藏启动 |
| `content` | `src/content.html` | 600 × 560 | 无边框、透明、置顶、隐藏启动 |

`settings` 和 `content` 的关闭请求会被后端拦截并改为隐藏，避免关闭后无法再次显示。`main` 在确认或取消后也会隐藏。

## AppState

`AppState` 包含：

| 字段 | 类型 | 用途 |
| --- | --- | --- |
| `settings` | `Mutex<Settings>` | 当前设置和快捷键值 |
| `previous_app_pid` | `Mutex<Option<i32>>` | 唤起输入框前的前台应用，用于粘贴后切回 |
| `registered_hotkeys` | `Mutex<Vec<String>>` | 当前已注册的全局快捷键，保存设置时可回滚 |
| `contents` | `Mutex<Vec<ContentEntry>>` | 用户主动维护的内容库 |

## Tauri 命令

### 设置

| 命令 | 作用 |
| --- | --- |
| `get_settings` | 返回当前 `Settings` |
| `save_settings` | 校验快捷键、注册全局快捷键、写入 `settings.json`、更新开机自启并广播设置更新 |

### 输入与粘贴

| 命令 | 作用 |
| --- | --- |
| `confirm_input` | 接收可选 `action`（`paste` / `paste_and_enter`，默认 `paste`），写入剪贴板、恢复目标焦点、粘贴并按本次动作可选回车，再恢复剪贴板 |
| `cancel_input` | 隐藏主窗口并切回唤起前的前台应用 |

### 窗口

| 命令 | 作用 |
| --- | --- |
| `open_settings` | 居中、显示并聚焦设置窗口 |
| `open_content` | 记录目标应用 PID、居中、显示并聚焦内容库 |

### 内容库

| 命令 | 作用 |
| --- | --- |
| `get_contents` | 按 `updated_at` 倒序返回内容 |
| `add_content` | 校验并新增单条内容，写入 `content.json` |
| `update_content` | 更新名称、内容、格式和更新时间 |
| `delete_content` | 删除指定 ID 的单条内容 |
| `paste_content` | 直接粘贴指定内容到打开内容库前的目标应用 |

## 后端事件

| 事件 | 方向 | 用途 |
| --- | --- | --- |
| `page-shortcuts-updated` | 后端 → 全部窗口 | 设置保存后通知前端刷新快捷键和透明度 |
| `reload-content` | 后端 → `content` | 内容变化或内容库打开时重新加载 |
| `paste-error` | 后端 → `main` | 粘贴失败后显示错误并重新显示窗口；payload 包含 `attempt_id`、`message`、`pasted`；已尝试粘贴的回车失败不恢复输入 |
| `paste-complete` | 后端 → `main` | 异步确认流程结束（成功或失败），含 `attempt_id`，解除前端工作状态 |
| `content-paste-error` | 后端 → `content` | 内容库直接粘贴失败后显示错误并重新显示内容库 |

## 全局快捷键

应用注册三个系统级全局快捷键：

- 唤起输入框：默认 `Alt+Space`
- 打开内容库：默认 `Alt+R`
- 打开设置：默认 `Alt+S`

确认和取消是页面内快捷键：

- 确认：默认 `Alt+Enter`
- 取消：默认 `Escape`

开启 `paste_and_send_enabled` 时，主输入窗口额外支持固定的 `Shift+Enter`，仅本次确认追加普通 Enter。保存设置时会把该组合键与五个应用快捷键一起检查冲突；实际注册到系统的始终只有前三个全局快捷键。

## 唤起和确认流程

### 唤起

1. 全局快捷键处理器在 Tauri 主线程读取当前前台应用 PID。
2. 将 PID 保存到 `AppState.previous_app_pid`。
3. 根据设置计算窗口位置和尺寸。
4. 显示并聚焦 `main` 窗口。

### 确认

1. 前端检查内容非空。
2. 后端再次检查内容非空和 macOS 辅助功能权限。
3. 读取设置，并取出 `previous_app_pid`。
4. 保存当前剪贴板文本和 HTML。
5. 按内容模式写入新剪贴板内容。
6. 隐藏 `main` 窗口。
7. 等待 400 ms，激活目标应用。
8. 等待 150 ms，再等待快捷键松开（最多 2 秒），确认目标 PID 仍在前台后发送粘贴快捷键。
9. 等待 480 ms；`paste_and_enter` 动作再次检查按键释放及目标 PID，然后模拟一次普通 Enter，再等待 80 ms 后才允许重新显示窗口。
10. 根据设置恢复原剪贴板；失败时发送带 `attempt_id`、`message`、`pasted` 的 `paste-error`。
11. 释放共享粘贴操作锁，发送 `paste-complete`；根据失败状态或 `keep_open_on_confirm` 重新显示主窗口。

`confirmation.rs` 提供动作解析、开关校验、独立可测试的时序/焦点检查和 RAII 粘贴锁；`platform::SystemInput` 适配实际系统按键。主输入框和内容库共用 `AppState.paste_in_progress`，避免重叠流程污染剪贴板。`pasted` 表示模拟粘贴事件已发出，而不是目标应用已确认接收；回车阶段失败不自动重新粘贴。

## 平台实现

### macOS

- 使用 `NSWorkspace` 获取前台应用。
- 使用 `NSRunningApplication` 激活目标应用。
- 使用 `AXIsProcessTrusted()` 检查辅助功能权限。
- 使用 Core Graphics 在 HID 层发送 `Command+V`。
- 应用启动后设置为 `ActivationPolicy::Accessory`。

### Windows

- 使用 `GetForegroundWindow` 和 `GetWindowThreadProcessId` 获取前台 PID。
- 枚举窗口并调用 `SetForegroundWindow`。
- 使用 `enigo` 发送 `Ctrl+V`。

当前 `platform.rs` 中存在重复的 Windows `FOUND_WINDOW`、`find_window_by_pid` 和 `activate_pid_windows` 定义。修改或声明 Windows 兼容性前，应先在 Windows 目标上执行 `cargo check`。

### Linux

- 使用 `xdotool` 获取前台 PID、查找窗口并激活。
- 使用 `enigo` 发送 `Ctrl+V`。
- Linux 分支存在，但产品文档主要目标是 macOS 和 Windows；Linux 支持需要通过目标桌面环境实际验证。

## 启动流程

1. 注册全局快捷键插件。
2. 初始化开机自启插件。
3. 初始化单实例插件；重复启动时聚焦已有主窗口。
4. 创建 `AppState`。
5. 加载 `settings.json` 和 `content.json`；首次升级时从旧 `history.json` 迁移手动片段。
6. 注册全局快捷键。
7. 创建托盘菜单。
8. 监听窗口关闭事件，将设置和内容库窗口关闭转换为隐藏。

## 托盘

托盘菜单包括：

- 打开输入框
- 内容库
- 设置
- 退出

托盘打开输入框前也会记录当前前台应用 PID。

## 相关文档

- [仓库结构](repository.md)
- [数据模型](data-model.md)
- [快速输入](../features/quick-input.md)
- [设置与快捷键](../features/settings-and-hotkeys.md)

## 粘贴并回车的平台细节

- macOS：用物理 HID 状态等待 Return 与修饰键释放；Core Graphics 私有事件源发送 Return 的按下/松开，显式清空修饰键标志。沿用辅助功能权限检查。
- Windows：`GetAsyncKeyState` 检查 Return 和修饰键释放，`enigo` 模拟普通 Return。
- Linux：通过 `x11rb` 查询 X11 按键映射与按下状态，`enigo` 模拟 Return。该路径依赖 X11，不能读取键盘状态时安全停止，不承诺原生 Wayland 支持。
- 自动测试使用模拟输入适配器，不会触发真实系统粘贴或发送消息；跨平台构建和目标应用实际发送仍需分别手动验证。
