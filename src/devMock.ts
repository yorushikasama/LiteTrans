// 开发期浏览器直开 devUrl 时的 Tauri mock(仅 DEV 且无 Tauri 环境时生效)
// 用于在普通 Chromium 里调试 UI 交互
export function installTauriMock() {
  const w = window as any;
  if (import.meta.env.PROD || w.__TAURI_INTERNALS__) return;

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
    transMode: "custom",
    sourceLang: "auto",
    targetLang: "English",
    hotkey: "alt+q",
    captureHotkey: "alt+x",
    customPrompt: "",
    autostart: false,
    autoCopy: true,
    autoTranslate: false,
    hotkeyEnabled: true,
    setupDone: true,
  };
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
          return Promise.resolve(null);
        case "set_active_profile":
          return Promise.resolve(null);
        case "get_presets":
          return Promise.resolve(presets);
        case "take_pending_text":
          // 弹卡入口用:给一段原文,便于在浏览器里看流式效果
          return Promise.resolve(
            document.body.dataset.page === "popup"
              ? "The five tones are gong, shang, jue, zhi and yu."
              : null,
          );
        case "take_pending_error":
          return Promise.resolve(null);
        case "toggle_pin":
          return Promise.resolve(true);
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
          // 弹卡/主窗口:按 StreamMsg 的形态推流,便于核对排版与光标
          mockStream(args?.onEvent);
          return Promise.resolve(null);
        default:
          return Promise.resolve(null);
      }
    },
  };

  /**
   * 模拟 Channel 流式消息(仅浏览器调试用)
   *
   * Channel 实例的 id 就是 transformCallback 返回的回调,消息形状是
   * { index, message };index 必须连续递增,否则会被当作乱序消息排队。
   */
  function mockStream(onEvent: any) {
    const send = typeof onEvent === "function" ? onEvent : onEvent?.id;
    if (typeof send !== "function") return;
    const out =
      "宫、商、角、徵、羽是中国传统音乐中的五声音阶,分别对应简谱里的 1、2、3、5、6,古人以此定调,也以此论声。";
    const chars = Array.from(out);
    let index = 0;
    const emit = (message: unknown) => send({ index: index++, message });
    emit({ type: "start", detected: "English", target: "Simplified Chinese", dict: false });
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
}
