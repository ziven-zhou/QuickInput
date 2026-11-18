# 数据模型

## Settings

`Settings` 定义于 `src-tauri/src/lib.rs`，序列化时使用 `snake_case` 字段名。

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `trigger_hotkey` | `Alt+Space` | 全局唤起输入框 |
| `content_hotkey` | `Alt+R` | 全局打开内容库 |
| `settings_hotkey` | `Alt+S` | 全局打开设置 |
| `confirm_hotkey` | `Alt+Enter` | 主输入窗口确认 |
| `cancel_hotkey` | `Escape` | 主输入窗口取消 |
| `window_position` | `center` | 主窗口位置，支持 `center`、`cursor` |
| `window_width` | `720` | 主窗口逻辑宽度，有效范围 400 到 1200 |
| `window_height` | `240` | 主窗口逻辑高度，有效范围 240 到 800 |
| `bg_opacity` | `0.97` | 背景透明度，有效范围 0.1 到 1.0 |
| `default_mode` | `plain` | 默认内容模式 |
| `pure_mode` | `false` | 是否只显示输入区域 |
| `input_font_size` | `14` | 输入框字号，有效范围 12 到 32 |
| `remember_window_position` | `false` | 是否保留用户拖动后的位置 |
| `window_x` | `null` | 主窗口上次位置的物理 X 坐标 |
| `window_y` | `null` | 主窗口上次位置的物理 Y 坐标 |
| `restore_clipboard` | `true` | 粘贴后是否恢复原剪贴板 |
| `autostart` | `false` | 是否登录时自动启动 |
| `keep_open_on_confirm` | `false` | 确认后是否保持主窗口打开 |
| `paste_and_send_enabled` | `false` | 是否启用主输入框 `Shift+Enter` 粘贴并回车，缺少字段时默认关闭 |

旧配置中的 `history_hotkey` 会通过 serde alias 迁移为 `content_hotkey`。旧 `history_enabled` 字段不再使用。

加载和保存时都会归一化窗口尺寸、透明度、弹出位置、默认内容模式和输入框字号。主窗口移动后会把坐标写入 `window_x`、`window_y`；坐标不在当前显示器可用区域时，恢复阶段回退到居中。

## ContentEntry

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `id` | `String` | 使用 Unix 纳秒时间戳生成 |
| `title` | `Option<String>` | 可选名称，空白值保存为 `null` |
| `text` | `String` | 用户主动保存的原始内容 |
| `mode` | `String` | `plain` 或 `markdown` |
| `created_at` | `u64` | 创建时间，Unix 毫秒 |
| `updated_at` | `u64` | 最后更新时间，Unix 毫秒 |

`get_contents` 按 `updated_at` 倒序返回内容。

新增和编辑要求文本非空，并拒绝完全相同的文本与格式组合。内容库不执行自动淘汰，也不对用户主动保存的内容做敏感信息过滤。

## 持久化位置

后端通过 Tauri `app.path().app_config_dir()` 获取应用配置目录，并创建以下文件：

```text
<app_config_dir>/settings.json
<app_config_dir>/content.json
```

### content.json

- 保存 `ContentEntry` 数组的格式化 JSON。
- 新增、编辑或删除单条内容后立即重写。
- 读取失败或 JSON 损坏时当前实现返回空内容库。

### 旧 history.json

首次启动且 `content.json` 不存在时，会读取旧 `history.json`：

- 只迁移 `kind = "snippet"` 的手动片段。
- `kind = "history"` 的自动记录不迁移。
- 旧记录中的模式经 `normalize_content_mode` 处理。
- 旧纳秒时间戳转换为毫秒。
- 原 `history.json` 保留为备份。

## 内容模式

| 模式 | 剪贴板内容 |
| --- | --- |
| `plain` | 纯文本 |
| `markdown` | 使用 Rust Markdown 渲染器生成 HTML，并保留纯文本备用内容 |

Markdown 链接只允许 `http://`、`https://` 和 `mailto:`。

## 相关文档

- [技术架构](architecture.md)
- [快速输入](../features/quick-input.md)
- [内容库](../features/content-library.md)
