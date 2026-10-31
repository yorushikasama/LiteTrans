# LiteTrans

轻量级 Windows AI 划词翻译。托盘常驻,选中文字按热键,光标旁弹出翻译卡片,AI 流式输出。

基于 Tauri 2 + Rust + WebView2:安装包约 5MB,托盘常驻内存约 40-80MB。

前端:UI 为 shadcn-vue(组件源码进仓库,基于 reka-ui 无头组件)+ Tailwind CSS v4 + Lucide 图标,深浅色跟随系统。组件已本地化,可自由修改 `src/components/ui/` 下任意实现。

## 功能(P1)

- **划词翻译**:选中文字后按全局热键(默认 `Alt+Q`),自动复制并弹卡翻译;弹卡启动 30s 后后台预热,首次划词零冷启动
- **截图 OCR 翻译**:按 `Alt+X` 框选屏幕任意区域(图片/PDF/不可选中文字的场景),Windows 内置 OCR 识别后弹卡翻译;框选热键与划词热键均可在设置中修改,托盘亦可发起
- **词典模式**:选中 1-3 个词自动切换为词典释义(音标/词性/例句),卡片带词典标识
- **自动复制译文**:弹卡翻译完成后自动写入剪贴板(设置可关),失焦不丢结果;错误卡片可直接「去设置」
- **翻译弹卡**:流式输出、复制、朗读(TTS)、钉住、失焦自动隐藏、`Esc` 关闭
- **主窗口翻译**:输入即译,支持长文本;关闭窗口 = 隐藏到托盘
- **AI 引擎**:多配置档案(CC-Switch 式),一键切换;支持三种接口协议:**OpenAI 兼容**(/chat/completions)、**Anthropic 兼容**(/v1/messages)、**Gemini 兼容**(generateContent);预设 DeepSeek / 智谱 GLM / Kimi / 通义千问 / 商汤日日新 / 硅基流动 / OpenAI / Claude / Gemini / OpenRouter / Ollama(本地)/ 自定义,选中预设自动填地址、模型与协议;每条配置带「测试」探活
- **翻译方向**:两种模式——「智能」自动识别源语言→固定译成中文;「自定义方向」源语言+目标语言各自指定(源语言也可选自动检测),显式指定时提示词写明 from X into Y;45 种目标语言,划词弹卡与主窗口共用同一设置
- **系统级能力**:OCR / TTS 用 Windows 内置组件,零额外体积
- 托盘菜单:显示主窗口 / 翻译剪贴板 / 设置 / 开机自启 / 退出
- API Key 存入 Windows 凭据管理器,不落盘

## 开发

```bash
npm install       # 前端依赖
npm run tauri dev # 开发运行(首次编译 Rust 依赖较慢)
```

## 构建

```bash
npm run tauri build   # 产物: src-tauri/target/release/bundle/nsis/
```

## 使用说明

1. 首次启动会打开主窗口,到「设置」选择服务商、填 API Key(本地 Ollama 无需 Key)
2. 在任意应用中选中文字,按 `Alt+Q` 弹出翻译卡片
3. 热键可在设置中修改(如 `alt+t`、`ctrl+shift+t`、`f9`)

## 已知限制(P1)

- 划词依赖模拟 `Ctrl+C`:部分应用(如某些终端)不支持,或目标应用以管理员运行时需 LiteTrans 同样提权
- 截图 OCR、翻译历史、鼠标划词自动弹标、悬浮球计划在 P2
- 部分推理型模型(如 deepseek-reasoner)输出思考内容,建议选非推理模型(如 deepseek-chat)
