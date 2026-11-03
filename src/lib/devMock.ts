// 开发期浏览器直开 devUrl 时的 Tauri mock(仅 DEV 且无 Tauri 环境时生效)
// 用于在普通 Chromium 里调试 UI 交互
// 返回 true 表示当前确实处在 mock 态,调用方可据此挂调试标识
export function installTauriMock(): boolean {
  const w = window as any;
  if (import.meta.env.PROD || w.__TAURI_INTERNALS__) return false;

  /** 浏览器调试态的设置在 localStorage 里的键 */
  const MOCK_KEY = "litetrans.mock.settings";

  const settings = {
    profiles: [
      {
        id: "p1",
        name: "DeepSeek 官方",
        provider: "deepseek",
        protocol: "openai",
        baseUrl: "https://api.deepseek.com/v1",
        model: "deepseek-chat",
        temperature: 0.3,
        apiKey: "sk-mock",
      },
      {
        id: "p2",
        name: "商汤 日日新",
        provider: "sensenova",
        protocol: "openai",
        baseUrl: "https://token.sensenova.cn/v1",
        model: "SenseChat-5",
        temperature: 0.3,
        apiKey: "",
      },
    ],
    activeId: "p1",
    // 与 Rust Settings::default() 保持一致:智能模式(自动识别 → 中文)。
    // 之前这里写的是 custom → English,截图里顶部选"英语"、译文却是中文,
    // 属于自造的假矛盾。
    transMode: "auto",
    sourceLang: "auto",
    targetLang: "Simplified Chinese",
    hotkey: "alt+q",
    captureHotkey: "alt+x",
    customPrompt: "",
    autostart: false,
    autoCopy: true,
    autoTranslate: false,
    hotkeyEnabled: true,
    theme: "system",
    setupDone: true,
  };
  // 读回上次调试时改过的设置(主要是主题),这样刷新页面不会跳回浅色
  try {
    const saved = localStorage.getItem(MOCK_KEY);
    if (saved) Object.assign(settings, JSON.parse(saved));
  } catch {
    /* 数据坏了就当没有,用默认 */
  }
  const presets = [
    {
      id: "deepseek",
      name: "DeepSeek",
      baseUrl: "https://api.deepseek.com/v1",
      model: "deepseek-chat",
      needKey: true,
      protocol: "openai",
      models: ["deepseek-chat", "deepseek-reasoner"],
    },
    {
      id: "sensenova",
      name: "商汤 日日新",
      baseUrl: "https://token.sensenova.cn/v1",
      model: "SenseChat-5",
      needKey: true,
      protocol: "openai",
      models: ["SenseChat-5"],
    },
    {
      id: "custom",
      name: "自定义",
      baseUrl: "",
      model: "",
      needKey: true,
      protocol: "openai",
      models: [],
    },
  ];

  w.__TAURI_INTERNALS__ = {
    transformCallback: (cb: unknown) => cb,
    invoke(cmd: string, args?: any) {
      switch (cmd) {
        case "get_settings":
          return Promise.resolve(JSON.parse(JSON.stringify(settings)));
        case "save_settings":
          // 真的把改动写回内存并落到 localStorage:否则改了主题、刷新一次
          // 就退回原值,调试时会误以为"设置没生效"。
          // (只存 mock 数据,不含真实 Key)
          if (args?.settings) {
            Object.assign(settings, args.settings);
            try {
              localStorage.setItem(MOCK_KEY, JSON.stringify(settings));
            } catch {
              /* 隐私模式下可能不可写,忽略 */
            }
          }
          return Promise.resolve(null);
        case "set_active_profile":
          return Promise.resolve(null);
        case "get_presets":
          return Promise.resolve(presets);
        case "take_pending_text":
          // 浏览器里没有真实的划词链路,返回 null 让弹卡显示空态,
          // 而不是塞一段假原文让人误以为划词成功了
          return Promise.resolve(null);
        case "take_pending_error":
          return Promise.resolve(null);
        case "toggle_pin":
          return Promise.resolve(true);
        case "get_runtime_info":
          return Promise.resolve({
            portable: false,
            dataDir: "(浏览器调试态,无真实数据目录)",
            version: "0.1.0",
          });
        case "test_profile":
          return Promise.resolve("连接正常 · 123 ms(mock)");
        case "get_history":
          return Promise.resolve([
            {
              ts: Math.floor(Date.now() / 1000),
              source: "Hello, LiteTrans!",
              result: "你好,LiteTrans!",
              direction: "自动 → 中文(简体)",
              dict: false,
              profile: "DeepSeek 官方",
              profileId: "p1",
            },
            {
              ts: Math.floor(Date.now() / 1000) - 3600,
              source: "ubiquitous",
              result: "adj. 无处不在的\n例句:Smartphones are ubiquitous today.",
              direction: "词典查询",
              dict: true,
              profile: "商汤 日日新",
              profileId: "p2",
            },
          ]);
        case "translate":
          // 弹卡/主窗口:按 StreamMsg 的形态推流,译文由真实原文派生
          mockStream(args?.onEvent, args?.text);
          return Promise.resolve(null);
        default:
          return Promise.resolve(null);
      }
    },
  };

  /**
   * 模拟 Channel 流式消息(仅浏览器调试用)
   *
   * 译文由传入的原文派生,并明确标注是调试生成的 —— 不预置任何"看起来像
   * 真译文"的假数据,从根上避免把调试产物当成真实输出。
   *
   * Channel 实例的 id 就是 transformCallback 返回的回调,消息形状是
   * { index, message };index 必须连续递增,否则会被当作乱序消息排队。
   */
  function mockStream(onEvent: any, text: string) {
    const send = typeof onEvent === "function" ? onEvent : onEvent?.id;
    if (typeof send !== "function") return;
    const src = (text ?? "").trim();
    // 方向必须跟着设置走:写死 "Simplified Chinese" 时,界面顶部选的是英语、
    // 读数条却显示"→ 中文",一眼就是矛盾的假数据
    const target = settings.transMode === "custom" ? settings.targetLang : "Simplified Chinese";
    const out = src
      ? `[浏览器调试] 这段译文由 mock 生成,不是真实模型输出。原文:${src}`
      : "[浏览器调试] 没有收到原文,请确认输入框里有内容。";
    const chars = Array.from(out);
    let index = 0;
    const emit = (message: unknown) => send({ index: index++, message });
    emit({
      type: "start",
      detected: "English",
      target,
      dict: false,
    });
    let i = 0;
    const t = setInterval(() => {
      const next = chars.slice(i, i + 2).join("");
      i += 2;
      if (next) emit({ type: "delta", text: next });
      if (i >= chars.length) {
        clearInterval(t);
        emit({ type: "done", text: out });
      }
    }, 55);
  }

  return true;
}
