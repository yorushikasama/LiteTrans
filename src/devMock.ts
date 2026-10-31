// 开发期浏览器直开 devUrl 时的 Tauri mock(仅 DEV 且无 Tauri 环境时生效)
// 用于在普通 Chromium 里调试 UI 交互
export function installTauriMock() {
  const w = window as any;
  if (import.meta.env.PROD || w.__TAURI_INTERNALS__) return;

  const settings = {
    profiles: [
      { id: "p1", name: "DeepSeek 官方", provider: "deepseek", protocol: "openai", baseUrl: "https://api.deepseek.com/v1", model: "deepseek-chat", temperature: 0.3, apiKey: "sk-mock" },
      { id: "p2", name: "商汤 日日新", provider: "sensenova", protocol: "openai", baseUrl: "https://token.sensenova.cn/v1", model: "SenseChat-5", temperature: 0.3, apiKey: "" },
    ],
    activeId: "p1",
    transMode: "custom",
    sourceLang: "auto",
    targetLang: "English",
    hotkey: "alt+q",
    customPrompt: "",
    autostart: false,
    autoCopy: true,
    setupDone: true,
  };
  const presets = [
    { id: "deepseek", name: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", model: "deepseek-chat", needKey: true, protocol: "openai", models: ["deepseek-chat", "deepseek-reasoner"] },
    { id: "sensenova", name: "商汤 日日新", baseUrl: "https://token.sensenova.cn/v1", model: "SenseChat-5", needKey: true, protocol: "openai", models: ["SenseChat-5"] },
    { id: "custom", name: "自定义", baseUrl: "", model: "", needKey: true, protocol: "openai", models: [] },
  ];

  w.__TAURI_INTERNALS__ = {
    transformCallback: (cb: unknown) => cb,
    invoke(cmd: string) {
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
          return Promise.resolve(null);
        case "toggle_pin":
          return Promise.resolve(false);
        case "test_profile":
          return Promise.resolve("连接正常 · 123 ms(mock)");
        case "translate":
          return Promise.resolve(null);
        default:
          return Promise.resolve(null);
      }
    },
  };
}
