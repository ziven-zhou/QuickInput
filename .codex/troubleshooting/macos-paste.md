# macOS 自动粘贴排查

## 问题现象

QuickInput 能唤起并接收确认，但目标应用没有收到粘贴内容。日志可能显示：

- 剪贴板已经成功写入。
- 前台应用已经切回目标应用。
- 合成按键事件已经发送。
- 最终内容仍未粘贴。

## 已知根因

历史问题记录确认，根因是 macOS TCC 辅助功能授权对象与实际运行应用不一致，而不是剪贴板或焦点切换失败。

典型情况：

- 用户授权的是 `/Applications/QuickInput.app`。
- 实际运行的旧进程来自 `src-tauri/target/release/bundle/macos/QuickInput.app`。
- 早期构建使用临时或 ad-hoc 签名身份，`Info.plist` 未绑定。
- macOS 15 将这些路径或签名身份视为不同应用。

后果：

- `AXIsProcessTrusted()` 返回 false。
- QuickInput 可能仍能调用 `CGEventPost`。
- 系统会静默丢弃没有辅助功能权限的合成按键。

## 当前代码行为

- `src-tauri/src/platform.rs` 使用 `AXIsProcessTrusted()` 检查权限。
- `confirm_input` 在权限检查失败时不执行粘贴，并返回中文错误提示。
- macOS 使用 Core Graphics 在 HID 层发送 `Command+V`。
- 当前代码仍通过 `write_paste_diag` 写入：

```text
/tmp/quickinput_paste_diag.log
```

这与[macOS 自动粘贴历史记录](macos-paste-case.md)中“已移除调试日志写入”的历史描述不一致。当前排障时以代码行为和该日志文件为准。

诊断行包含：

- `ax_trusted`
- `target_pid`
- `front_after_activate`
- `front_after_paste`
- `result`
- 主输入框流程还包含 `action`（`Paste` / `PasteAndEnter`）；失败时附 `pasted`，区分粘贴前失败和回车阶段失败。

## 优先检查

### 1. 确认运行的是哪个应用

```bash
ps -axo pid,comm,args | rg 'QuickInput|quickinput'
```

确认进程来自 `/Applications/QuickInput.app`，而不是项目构建目录。

### 2. 检查签名身份

```bash
codesign -dv --verbose=4 /Applications/QuickInput.app
```

期望信息包括：

```text
Identifier=com.quickinput.quickinput
```

同时确认 `Info.plist` 已绑定，而不是仅显示临时标识。

### 3. 检查辅助功能授权

打开：

```text
系统设置 → 隐私与安全性 → 辅助功能
```

确认授权对象是正在使用的 `/Applications/QuickInput.app`。如果列表中存在多个 QuickInput 条目，应删除错误路径对应项，并重新添加当前应用。

### 4. 查看诊断日志

```bash
tail -n 50 /tmp/quickinput_paste_diag.log
```

关键判断：

- `ax_trusted=false`：辅助功能权限或授权对象不匹配。
- `target_pid=0`：没有捕获到目标应用。
- `front_after_activate` 与目标 PID 不一致：焦点切换失败。
- `front_after_paste` 正确但目标应用未粘贴：优先检查权限和签名身份。
- `result=ok` 但页面仍无内容：继续检查目标应用的粘贴行为和剪贴板格式。

## 恢复流程

1. 完全退出正在运行的 QuickInput。
2. 重新执行 `npm run tauri build`。
3. 按项目现有流程签名并替换 `/Applications/QuickInput.app`。
4. 确认旧进程已退出，再启动新应用。
5. 在辅助功能设置中确认授权对象为 `/Applications/QuickInput.app`。
6. 在文本编辑器中手动测试唤起、确认和粘贴。

常用构建与检查命令：

```bash
cd /Users/shmizhouzhiyuan/InputDialog/src-tauri
cargo check

cd /Users/shmizhouzhiyuan/InputDialog
npm run build
npm run tauri build
codesign --force --deep --sign - --entitlements src-tauri/entitlements.plist \
  src-tauri/target/release/bundle/macos/QuickInput.app
ditto src-tauri/target/release/bundle/macos/QuickInput.app /Applications/QuickInput.app
open /Applications/QuickInput.app
```

替换 `/Applications/QuickInput.app` 前必须确认路径和旧进程状态。`ditto` 覆盖安装前，项目中另有一套先删除旧 `.app` 再复制产物的流程，执行时同样需要谨慎。

## 保留约束

- 日常只从 `/Applications/QuickInput.app` 启动。
- 重新打包替换应用前，完全退出旧进程。
- 每次替换 `.app` 后都要确认辅助功能授权仍指向当前应用。
- bundle identifier 保持为 `com.quickinput.quickinput`，不要以 `.app` 结尾。
- 粘贴逻辑应继续保留在主线程派发路径，避免 macOS UI 和事件 API 的线程问题。

## 相关文件

- `src-tauri/src/platform.rs`：权限检查、前台应用识别、应用激活和粘贴模拟。
- `src-tauri/src/lib.rs`：`confirm_input` 流程、剪贴板快照与恢复、诊断日志。
- `src-tauri/tauri.conf.json`：bundle identifier 和打包配置。
- `src-tauri/entitlements.plist`：macOS entitlements。

## 相关文档

- [快速输入](../features/quick-input.md)
- [技术架构](../structure/architecture.md)
- [安全与发布](../rules/security-release.md)
- [macOS 自动粘贴历史记录](macos-paste-case.md)

## 粘贴并发送未生效

1. 确认已开启“启用粘贴并发送快捷键”，并在 QuickInput 内使用 `Shift+Enter`；原确认快捷键和按钮不会追加回车。
2. 确认目标应用手动按普通 Enter 可以发送。功能不识别应用的自定义发送组合键。
3. 快捷键需要松开；持续按住 Enter、Shift、Option 等超过等待上限会安全中止。
4. 粘贴前焦点变化会取消粘贴；粘贴后焦点变化会跳过回车。
5. 出现“已尝试粘贴，未完成回车”时先检查目标窗口，不要直接重复确认；后端不会自动重跑粘贴。
6. 同一应用内聊天切换不一定改变 PID，此保护不能代替人工确认目标会话。
