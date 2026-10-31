# LiteTrans

轻量级 Windows AI 划词翻译。托盘常驻,选中文字按热键,光标旁弹出翻译卡片,AI 流式输出。

基于 Tauri 2 + Rust + WebView2:安装包约 3MB,托盘常驻内存约 40-80MB。

前端 UI 为 shadcn-vue(组件源码进仓库,基于 reka-ui 无头组件)+ Tailwind CSS v4 + Lucide 图标,深浅色跟随系统。组件已本地化,可自由修改 `src/components/ui/` 下任意实现。

## 功能

- **划词翻译**:选中文字后按全局热键(默认 `Alt+Q`),自动复制并弹卡翻译;弹卡启动 30s 后后台预热,首次划词零冷启动
- **截图 OCR 翻译**:按 `Alt+X` 框选屏幕任意区域(图片/PDF/不可选中文字的场景),Windows 内置 OCR 识别后弹卡翻译;热键可改,托盘亦可发起
- **主窗口翻译**:输入即译(可选防抖自动)、清空/粘贴、译文朗读(TTS)、翻译历史(最近 50 条)
- **AI 引擎**:多配置档案(CC-Switch 式),一键切换;支持三种接口协议:**OpenAI 兼容**(/chat/completions)、**Anthropic 兼容**(/v1/messages)、**Gemini 兼容**(generateContent);预设 DeepSeek / 智谱 GLM / Kimi / 通义千问 / 商汤日日新 / 硅基流动 / OpenAI / Claude / Gemini / OpenRouter / Ollama(本地)/ 自定义,选中预设自动填地址、模型与协议;每条配置带「测试」探活与「在线获取」模型列表
- **翻译方向**:智能模式(自动识别源语言→中文)与自定义方向(源+目标各自指定,45 种语言),弹卡与主窗口共用
- **词典模式**:短词自动切换词典释义(音标/词性/例句)
- **输出格式**:提示词硬规则(禁止开场白/包裹引号/结构保真)+ 引擎层输出净化
- **系统级能力**:OCR / TTS 用 Windows 内置组件,零额外体积
- 托盘菜单:显示主窗口 / 翻译剪贴板 / 截图翻译 / 配置切换 / 开机自启 / 划词热键开关 / 退出
- **安全**:API Key 存入 Windows 凭据管理器,配置文件不落盘

## 项目结构

```
├─ src/                    前端 (Vue 3 + TypeScript)
│  ├─ App.vue              主窗口:翻译 / 历史 / 设置
│  ├─ Popup.vue            划词弹卡
│  ├─ components/          LanguageSelect / ModelCombobox / TranslateDirection
│  ├─ components/ui/       shadcn-vue 组件源码(本地化,可自由修改)
│  └─ assets/index.css     Tailwind v4 主题(深浅色跟随系统)
├─ src-tauri/src/          Rust 后端
│  ├─ lib.rs               应用组装、全局状态、热键注册
│  ├─ tray.rs              托盘图标与菜单
│  ├─ engine.rs            AI 引擎(三协议 SSE + 输出净化)
│  ├─ clipboard_flow.rs    划词捕获 → 弹卡
│  ├─ capture_flow.rs      截图框选窗口
│  ├─ ocr.rs               GDI 抓屏 + Windows OCR
│  ├─ settings.rs          配置持久化(含旧版迁移)+ 服务商预设
│  ├─ history.rs           翻译历史(本地 JSON,50 条上限)
│  ├─ lang.rs              语种识别与方向解析
│  ├─ commands.rs          Tauri IPC 命令
│  ├─ input.rs             按键模拟 / 光标 / 剪贴板
│  └─ tts_win.rs           系统 TTS
└─ scripts/                工具脚本(图标生成 / shadcn 组件拉取)
```

## 开发

```bash
npm install        # 前端依赖
npm run tauri dev  # 开发运行(首次编译 Rust 依赖较慢)
```

浏览器直开 `http://localhost:1420` 也可以调 UI(DEV 模式自动 mock 后端数据)。

## 测试与规范

```bash
cd src-tauri
cargo fmt --all        # 格式化
cargo clippy -- -D warnings   # lint
cargo test             # 单元测试
```

```bash
npm run lint:format    # prettier 检查
npm run format         # prettier 格式化
```

提交前请确保 `cargo fmt/clippy/test` 与 `npm run build` 全部通过(CI 会检查)。

## 构建

```bash
npm run tauri build   # 产物: src-tauri/target/release/bundle/nsis/
```

## 使用说明

1. 首次启动会打开主窗口,到「设置」添加模型配置(选服务商 → 填 Key → 在线获取模型 → 测试)
2. 在任意应用中选中文字,按 `Alt+Q` 弹出翻译卡片;单词自动进入词典模式
3. 按 `Alt+X` 框选屏幕区域,识别并翻译图片/PDF 中的文字
4. 热键、方向、自动复制、输入即译等均在设置中可调

## 已知限制

- 划词依赖模拟 `Ctrl+C`:部分应用(如某些终端)不支持,或目标应用以管理员运行时需 LiteTrans 同样提权
- 截图 OCR 目前仅支持主显示器(多屏混合 DPI 需要额外处理)
- 部分推理型模型(如 deepseek-reasoner)输出思考内容,建议选非推理模型
- OCR 精度依赖系统语言包(Windows 设置 → 时间和语言 → 语言)
