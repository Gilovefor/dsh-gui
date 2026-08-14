# dsh-gui

桌面壳：把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) 的本地 Web UI 包成原生桌面窗口（Tauri v2）。

后端是 `dsh web`（本地 Node 服务，自选空闲端口），壳只负责「起进程 → 就绪后加载 → 退出时清理」，不耦合 harness 内部协议。

## 前置要求

- Node.js ≥ 20（运行 harness 需要）
- Rust stable toolchain（构建需要；`rustup default stable`）
- Windows 11（自带 WebView2）—— Tauri 也支持 macOS / Linux，但本项目按 Windows 编写

## 安装

```bash
rustup default stable        # 首次
npm install                  # 安装 @tauri-apps/cli 和 @deepseek-ai/dsh
npm run icon -- app-icon.svg # 从 DeepSeek 鲸鱼 SVG 生成 src-tauri/icons/ 全套（一次性）
```

## 开发 / 构建

```bash
npm run dev     # 开发模式：窗口 + 热重载（首次编译约几分钟）
npm run build   # 发布：release .exe（src-tauri/target/release/dsh-gui.exe）
npm run bundle  # 可选：额外产出 NSIS/MSI 安装包
```

> 注：v1 的壳不打包 Node 运行时与 node_modules，exe 需在仓库目录内运行（由项目内 `node_modules/@deepseek-ai/dsh` 提供后端）。打包分发留待后续增强。

## 运行说明

- 每次启动自选空闲端口（不占用默认 3080，不会和手动跑的 `dsh web` 冲突）。
- 后端 stdout/stderr 写到项目根目录 `dsh-gui-node.log`。
- 关闭窗口即退出并 `taskkill /T /F` 清理整个 node 进程树。
- WebView2 的存储与系统 Edge 隔离：首次在窗口内登录 / 信任属于预期行为。
- 环境变量 `DSH_CMD` 可覆盖 dsh CLI 入口（默认 `node_modules/@deepseek-ai/dsh/lib/bin.js`），便于测试其他版本。
