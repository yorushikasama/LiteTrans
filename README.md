# LiteTrans

轻量级 Windows AI 划词翻译。托盘常驻,选中文字按热键,光标旁弹出翻译卡片,AI 流式输出。

基于 Tauri 2 + Rust + WebView2:安装包约 3MB,托盘常驻内存约 40-80MB。

前端 UI 为 shadcn-vue(组件源码进仓库,基于 reka-ui 无头组件)+ Tailwind CSS v4 + Lucide 图标,深浅色跟随系统。组件已本地化,可自由修改 `src/components/ui/` 下任意实现。

## 主题:宫商角徵羽

五音对到功能语义上,不做装饰性堆叠(无米黄纸底、朱印、毛笔插画、渐变金字):

| 五音 | 色相 | 承担的语义 |
|---|---|---|
| **宫** | 土黄 | 主操作、当前选中、流式光标、选框 |
| **商** | 金白 | 结构线、分组标题、中性读数 |
| **角** | 木青 | 成功、已复制、词典标记 |
| **徵** | 火赤 | 错误、破坏性操作 |
| **羽** | 水墨 | 信息、深色层、条目底色 |

界面语汇是一块**调音台**:五根竖柱的立标(顶栏)、等宽字读数、发丝分隔线、按行展开的设置项。

**调式身份**:每条模型配置按 id 派生一个「调」(宫调/商调/角调/徵调/羽调),设置列表、历史记录、翻译弹卡与托盘菜单显示同一个字,所以任何地方看到「商」都知道在用哪条配置。取模算法在 Rust(`settings::tone_index`)与 TS(`src/tones.ts`)两侧一致,各有一个断言测试锁住。

所有颜色 token 在 `src/assets/index.css`,用 OKLCH;深色模式的次要文字对比度 ≥4.5:1(有脚本核算过)。

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

前端按「入口 → 页面 → 组件/逻辑」四层切分,一处改动只落在一个层次里:

```
src/
├─ entries/            各窗口入口,只做挂载
│  ├─ main.ts          主窗口
│  ├─ popup.ts         划词弹卡
│  └─ capture.ts       截图框选层(纯 TS,无框架)
├─ views/             页面级组件,只管渲染与事件转发
│  ├─ TranslateView.vue
│  ├─ HistoryView.vue
│  ├─ SettingsView.vue
│  └─ PopupView.vue    弹卡整页(窗口小,不再拆子页面)
├─ layout/SideBar.vue 主窗口侧栏
├─ components/        可复用组件
│  ├─ settings/       设置页的三块 section
│  ├─ ToneMark.vue    五音立标(高亮当前调)
│  ├─ ToneChip.vue    调式标签(单个宫/商/角/徵/羽)
│  ├─ LanguageSelect.vue / ModelCombobox.vue / TranslateDirection.vue
│  └─ ui/             shadcn-vue 组件源码(本地化,可自由修改)
├─ composables/       页面逻辑(状态 + 请求),与视图一一对应
│  ├─ useTranslate.ts 流式翻译、剪贴板、朗读、输入即译
│  └─ useSettings.ts  设置与模型档案增删改查、连通性测试
├─ theme/tones.ts     调式派生算法(与 Rust settings::tone_index 必须一致)
├─ lib/               横切工具(errorBanner / devMock / cn)
├─ api.ts             与 Rust 通信的唯一入口(全部命令的类型封装)
├─ types.ts          共享类型与语言表
└─ assets/index.css   Tailwind v4 主题(深浅色跟随系统)
```

Rust 侧:

```
src-tauri/src/
├─ lib.rs              应用组装、全局状态、热键注册
├─ tray.rs             托盘图标与菜单
├─ engine.rs           AI 引擎(三协议 SSE + 输出净化)
├─ clipboard_flow.rs   划词捕获 → 弹卡
├─ capture_flow.rs     截图框选窗口
├─ ocr.rs              GDI 抓屏 + Windows OCR
├─ settings.rs         配置持久化(含旧版迁移)+ 服务商预设
├─ paths.rs            数据目录判定(便携版 vs 安装版)
├─ history.rs          翻译历史(本地 JSON,50 条上限)
├─ lang.rs             语种识别与方向解析
├─ commands.rs         Tauri IPC 命令
├─ input.rs            按键模拟 / 光标 / 剪贴板
└─ tts_win.rs          系统 TTS
```

### 几条硬约定(由 `npm run lint:structure` 强制)

1. **单文件 ≤ 450 行**。超过就该拆,而不是继续往里加。
2. **前端不直接调用 `invoke`**。所有命令走 `src/api.ts`:
   后端改名时 TypeScript 能立刻报错,而不是等运行时炸。
3. **`views/` 与 `components/` 只放 `.vue`**。逻辑放 `composables/`。

## 开发

```bash
npm install        # 前端依赖
npm run tauri dev  # 开发运行(首次编译 Rust 依赖较慢)
```

浏览器直开 `http://localhost:1420` 调 UI 时,DEV 模式会自动 mock 后端数据,
页面右下角会显示「浏览器调试」角标,标题带 `[调试]` 前缀 —— 看到它就说明
译文不是真实模型输出。

## 测试与规范

```bash
npm run lint           # prettier + 类型 + 目录结构,一次跑完
npm run check          # 上面这些 + Rust 单元测试
```

```bash
cd src-tauri
cargo fmt --all                # 格式化
cargo clippy -- -D warnings    # lint
cargo test                     # 单元测试
```

布局审计(需要先 `npm run dev`):

```bash
npm run audit:ui       # 5 个窗口尺寸 × 3 个页面,查溢出/点击目标/文本截断
npm run audit:popup    # 弹卡 420x330 专项
```

`audit:ui` 的第一个尺寸是 **533×400**,对应「800×600 窗口 + 150% 缩放」——
那是本机真实的默认尺寸,比 `tauri.conf.json` 里的 `minWidth/minHeight` 还小,
是最容易被忽略的一档。

提交前请确保 `npm run check` 与 `npm run build` 全部通过。

## 构建

```bash
npm run tauri build      # 安装版,产物: src-tauri/target/release/bundle/nsis/
bash scripts/make-portable.sh   # 便携版 zip,产物: release/LiteTrans_0.1.0_x64_portable.zip
```

## 安装版与便携版

两种版本共用同一个 `litetrans.exe`,区别只在**程序旁边有没有 `portable.txt`**:

| | 安装版 | 便携(绿色)版 |
|---|---|---|
| 数据位置 | `%APPDATA%\LiteTrans` | `<程序目录>\data\`(自动创建) |
| 开机自启 | 支持(写注册表) | 不提供,托盘里也不显示该项 |
| 卸载 | 走卸载程序 | 删掉整个文件夹 |
| 迁移 | 需手动导出配置 | 整个文件夹拷走即可 |

用「标记文件」而不是「检测目录是否可写」来判定:后者在 `C:\Program Files` 下会因
UAC 虚拟化产生"看似可写、实则被重定向"的结果,判定不可靠。

**API Key 两种情况都不随目录迁移**,它存在 Windows 凭据管理器(按服务名 `LiteTrans`
+ 配置 id),换机器或换用户需要重新填一次。这是有意的:凭据管理器是这台机器的安全边界。

设置页底部「关于」会显示当前是哪种版本、数据目录的实际路径,并提供「打开数据目录」。

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
