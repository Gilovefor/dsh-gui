# dsh-gui

桌面壳：把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) 的本地 Web UI 包成原生桌面窗口（Tauri v2）。

- **架构**：一个"笨壳"——自选空闲端口拉起 `dsh web` 本地 Node 服务 → 轮询就绪 → WebView 加载 → 退出时 `taskkill /T /F` 清理进程树。不耦合 harness 内部协议，上游更新不易坏。
- **图标**：DeepSeek 官方鲸鱼，源文件 `app-icon.svg`（取自 harness web 前端的 favicon，矢量可无损放大）。

## 前置要求

- **Node.js ≥ 22.19（硬性要求）**：harness 依赖的 `@earendil-works/pi-ai` 要求 node ≥ 22.19，低于此版本时 WebView 会报 "Failed to load plugins / 34 entries did not activate"。推荐放一个便携 Node 到项目 `node-runtime/`，壳会优先用它；没有则回退系统 `node`。
- Rust stable toolchain（构建需要；`rustup default stable`）
- Windows 11（自带 WebView2）—— Tauri 也支持 macOS / Linux，但本项目按 Windows 编写

## 安装

```bash
rustup default stable        # 首次
npm install                  # 安装 @tauri-apps/cli 和 @deepseek-ai/dsh
```

便携 Node（推荐，≥22.19）：从 <https://npmmirror.com/mirrors/node/>（国内快）或 <https://nodejs.org/dist/> 下载 `node-v22.x.x-win-x64.zip`，解压后把**整个 `node-v22.x.x-win-x64` 目录**放到项目 `node-runtime/`（即 `node-runtime/node.exe` 存在即可）。

改图标时重新生成全套：`npm run icon -- app-icon.svg`

## 开发 / 构建

```bash
npm run dev     # 开发模式：窗口 + 热重载（首次编译约几分钟）
npm run build   # 发布：release .exe（src-tauri/target/release/dsh-gui.exe）
npm run bundle  # 可选：额外产出 NSIS/MSI 安装包
```

> 注：v1 不打包 Node 运行时与 node_modules，exe 需在仓库目录内运行（由项目内 `node_modules/@deepseek-ai/dsh` 提供后端）。打包分发留待后续增强。

## 运行说明

- **无终端窗口**：node 服务以 `CREATE_NO_WINDOW` 后台运行，stdout/stderr 写入 `dsh-gui-node.log`。
- 每次启动自选空闲端口（不占用默认 3080，不会和手动跑的 `dsh web` 冲突）。
- 关闭窗口即退出并 `taskkill /T /F` 清理整个 node 进程树。
- 外链（网页链接）自动交给系统浏览器。
- WebView2 的存储与系统 Edge 隔离：首次在窗口内登录 / 信任属于预期行为。
- 环境变量 `DSH_CMD` 可覆盖 dsh CLI 入口（默认 `node_modules/@deepseek-ai/dsh/lib/bin.js`），便于测试其他版本。

## 常见问题

- **窗口里报 "Failed to load plugins / web boot: 34 entries did not activate"** → 基本是 node 版本低于 22.19。放一个 ≥22.19 的便携 node 到 `node-runtime/`，或升级系统 node。
- **`npm run build` 报"拒绝访问"** → 之前的 dsh-gui 实例还在运行锁住了 exe，先关掉窗口再编译。
- **git 推送失败** → 本机 git 全局配了 `http.proxy=127.0.0.1:7890`，代理工具未启动时会连不上；本项目远程用的是 SSH（`git@github.com:Gilovefor/dsh-gui.git`），不受该代理影响。
