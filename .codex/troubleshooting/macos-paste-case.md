# QuickInput 自动粘贴问题（macOS 15）

> 本文由根目录原 `PROBLEM_PASTE.md` 迁移整理，记录当时的故障现象、根因和修复结论。
>
> 当前排查流程和代码现状以[macOS 自动粘贴排查](macos-paste.md)为准。

## 当前状态

已解决。

最终可用路径是：

1. QuickInput 将文本写入剪贴板。
2. 隐藏输入窗口。
3. 在 macOS 主线程激活原目标应用。
4. 发送一次 HID 层 `Cmd+V`。

## 根因

问题不是剪贴板写入，也不是目标应用焦点切回失败。排查日志确认：发送时前台应用已经是 `Transsioner`，`Cmd+V` 事件也已经发出。

真正原因是 macOS TCC 辅助功能权限授权对象和实际运行的 QuickInput 不一致：

- 用户授权/安装的是 `/Applications/QuickInput.app`。
- 实际运行过的旧进程曾来自 `src-tauri/target/release/bundle/macos/QuickInput.app`。
- 早期构建还是临时 ad-hoc 签名身份，`codesign` 显示 `Identifier=quickinput-...`、`Info.plist=not bound`。

macOS 15 会把这些路径和签名身份视为不同应用。结果是 `AXIsProcessTrusted` 为 false，即使 `CGEventPost` 调用返回成功，系统也会静默丢弃合成按键。

## 已修正

- `src-tauri/tauri.conf.json` 的 bundle identifier 改为 `com.quickinput.quickinput`，避免以 `.app` 结尾。
- `/Applications/QuickInput.app` 已替换为同一份构建产物，并重新 ad-hoc 签名，`codesign` 显示 `Identifier=com.quickinput.quickinput` 且 `Info.plist` 已绑定。
- `confirm_input` 保留主线程派发粘贴逻辑，避免 macOS UI/事件 API 在线程切换上不稳定。
- 移除了调试期的 `/tmp/quickinput_*.log` 写入、AX 诊断、逐字符 Unicode 输入、旧备份文件。

## 保留的实现约束

- 日常只从 `/Applications/QuickInput.app` 启动。
- 如果重新打包并替换 app，必须完全退出旧进程后再打开新 app。
- 如果粘贴再次失效，优先确认“系统设置 -> 隐私与安全性 -> 辅助功能”里授权的是 `/Applications/QuickInput.app`，不是项目构建目录里的 app。

## 关键文件

- `src-tauri/src/platform.rs`：平台相关的焦点切换和粘贴模拟。
- `src-tauri/src/lib.rs`：`confirm_input` 流程，负责写剪贴板、隐藏窗口、主线程粘贴、恢复剪贴板。
- `src-tauri/tauri.conf.json`：bundle identifier 和打包配置。
- `src-tauri/entitlements.plist`：macOS entitlements。

## 验证命令

```bash
cd /Users/shmizhouzhiyuan/InputDialog/src-tauri
cargo check

cd /Users/shmizhouzhiyuan/InputDialog
npm run build
npm run tauri build
codesign --force --deep --sign - --entitlements src-tauri/entitlements.plist src-tauri/target/release/bundle/macos/QuickInput.app
ditto src-tauri/target/release/bundle/macos/QuickInput.app /Applications/QuickInput.app
```

## 文档关系

- 当前排查步骤：[macOS 自动粘贴排查](macos-paste.md)
- 输入与粘贴流程：[快速输入](../features/quick-input.md)
- 项目知识总入口：[MOC](../MOC.md)
