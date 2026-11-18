# 安全与发布

## 敏感数据

- 禁止提交或外发密钥、凭证、Token、证书、未公开配置和个人剪贴板内容。
- 不要将本地核心源码、数据库、环境文件或包含密码和访问凭证的内容发送到外部 API、聊天工具或其他网络端点。
- 不得把个人内容库条目或剪贴板数据写入日志、测试夹具或仓库文件。
- 内容库只在用户主动新增或编辑时写入，不自动记录粘贴内容。
- 用户主动保存的敏感内容仍会以本地明文存储，不能把内容库视为敏感信息保护机制。

## 输入处理

- 空文本不得进入粘贴或内容库保存流程。
- Rust 后端对快捷键进行格式校验和冲突校验。
- 前端窗口尺寸控件有范围和默认值，但后端不应依赖前端校验作为唯一安全边界。
- Markdown 链接只允许 `http://`、`https://` 和 `mailto:`，并过滤控制字符和 HTML 尖括号。
- 修改持久化格式时，要处理旧数据缺失字段、空文件和损坏 JSON。

## macOS 权限

自动粘贴依赖 macOS 辅助功能权限：

- 实际运行的 QuickInput 必须被“系统设置 → 隐私与安全性 → 辅助功能”授权。
- 应优先从 `/Applications/QuickInput.app` 启动并授权。
- 不同路径或不同签名身份的 `.app` 被视为不同授权对象。
- 当前 bundle identifier 为 `com.quickinput.quickinput`。
- `platform::can_simulate_paste()` 使用 `AXIsProcessTrusted()` 判断权限；未授权时 `confirm_input` 直接返回错误。

详细排查见[macOS 自动粘贴排查](../troubleshooting/macos-paste.md)。

## 开发与生产构建

开发模式：

```bash
npm run tauri dev
```

前端构建：

```bash
npm run build
```

生产构建：

```bash
npm run tauri build
```

生产产物位于：

```text
src-tauri/target/release/bundle/macos/QuickInput.app
```

安装到本机前应先完全退出旧进程，再替换 `/Applications/QuickInput.app`。现有项目流程如下：

```bash
rm -rf /Applications/QuickInput.app
cp -R src-tauri/target/release/bundle/macos/QuickInput.app /Applications/QuickInput.app
open /Applications/QuickInput.app
```

`rm -rf` 属于高风险操作。执行前必须确认目标路径准确，并确认 QuickInput 已退出。

历史问题记录中还使用了签名和覆盖安装流程：

```bash
codesign --force --deep --sign - --entitlements src-tauri/entitlements.plist \
  src-tauri/target/release/bundle/macos/QuickInput.app
ditto src-tauri/target/release/bundle/macos/QuickInput.app /Applications/QuickInput.app
```

## 发布检查

- 确认前端构建成功。
- 确认 Rust 后端通过 `cargo check`。
- 确认 `/Applications/QuickInput.app` 是本次新构建的产物。
- 确认旧进程没有继续运行。
- 确认辅助功能权限指向正在使用的 `/Applications/QuickInput.app`。
- 手动验证唤起、确认粘贴、取消、内容库直接粘贴、设置和托盘入口。
- 修改 Windows 或 Linux 相关代码时，应在目标平台单独构建和验证，不能只依赖 macOS 构建结果。

## 相关文档

- [工程规范](engineering.md)
- [技术架构](../structure/architecture.md)
- [macOS 自动粘贴排查](../troubleshooting/macos-paste.md)
