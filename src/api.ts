/**
 * 与 Rust 后端通信的唯一入口。
 *
 * 之前每个组件直接写 `invoke("save_settings", ...)`,命令名是裸字符串:
 * 后端改名时 TypeScript 完全不会报错,只能等运行时才炸。这里把全部命令
 * 收敛成带类型的方法,新增/改名命令只需改这一个文件。
 */
import { Channel, invoke } from "@tauri-apps/api/core";
import type { HistoryItem, Preset, Profile, RuntimeInfo, Settings, StreamMsg } from "@/types";

/** 翻译事件回调 */
export type StreamHandler = (msg: StreamMsg) => void;

export const api = {
  /* ── 设置 ── */
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  getPresets: () => invoke<Preset[]>("get_presets"),

  /* ── 模型配置档案 ── */
  /** 切换当前启用的配置(Rust 端会同时把热键/托盘状态同步过去) */
  setActiveProfile: (id: string) => invoke<void>("set_active_profile", { id }),
  /** 测试连通性,返回后端给的可读结论 */
  testProfile: (profile: Profile) => invoke<string>("test_profile", { profile }),
  /** 在线拉取该配置可用的模型名 */
  fetchModels: (profile: Profile) => invoke<string[]>("fetch_models", { profile }),

  /* ── 翻译 ── */
  /**
   * 发起翻译。事件通过 Channel 流式回传,不要 await 它的返回值来等结果。
   * @param sourceWindow "main" | "popup",用于后端按窗口做取消隔离
   */
  translate: (text: string, sourceWindow: string, onEvent: StreamHandler) => {
    const channel = new Channel<StreamMsg>();
    channel.onmessage = onEvent;
    return invoke<void>("translate", { text, source: sourceWindow, onEvent: channel });
  },
  cancelTranslate: (sourceWindow: string) =>
    invoke<void>("cancel_translate", { source: sourceWindow }),

  /* ── 历史 ── */
  getHistory: () => invoke<HistoryItem[]>("get_history"),
  clearHistory: () => invoke<void>("clear_history"),

  /* ── 剪贴板 ── */
  readClipboard: () => invoke<string | null>("read_clipboard"),
  writeClipboard: (text: string) => invoke<void>("write_clipboard", { text }),

  /* ── 语音朗读 ── */
  ttsSpeak: (text: string) => invoke<void>("tts_speak", { text }),
  ttsStop: () => invoke<void>("tts_stop"),

  /* ── 窗口 ── */
  hidePopup: () => invoke<void>("hide_popup"),
  /** 钉住弹卡(钉住后失焦不隐藏),返回新的钉住状态 */
  togglePin: () => invoke<boolean>("toggle_pin"),
  openDataDir: () => invoke<void>("open_data_dir"),
  /** 把文本带回主窗口打开 */
  openMainWithText: (text: string) => invoke<void>("open_main_with_text", { text }),
  openMainSettings: () => invoke<void>("open_main_settings"),

  /* ── 弹卡首帧 ── */
  /** 弹卡页面刚挂载时主动取后端暂存的文本(首次创建时事件还没到) */
  takePendingText: () => invoke<string | null>("take_pending_text"),
  takePendingError: () => invoke<string | null>("take_pending_error"),

  /* ── 运行环境 ── */
  getRuntimeInfo: () => invoke<RuntimeInfo>("get_runtime_info"),

  /* ── 截图翻译(capture 窗口) ── */
  hideCapture: () => invoke<void>("hide_capture"),
  /** 把框选区域交给后端做 OCR + 翻译。
      坐标是**物理像素**(调用方自行乘 devicePixelRatio)。 */
  ocrTranslate: (rect: { x: number; y: number; w: number; h: number }) =>
    invoke<void>("ocr_translate", rect),
};
