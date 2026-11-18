#!/bin/bash
# QuickInput 启动脚本 - 开发模式
cd "$(dirname "$0")"
echo ">> 启动 QuickInput 开发模式..."
echo ">> 提示: 按 ⌥Space 唤出输入框，Enter 发送，Esc 取消"
echo ">> 按 Ctrl+C 停止"
npm run tauri dev
