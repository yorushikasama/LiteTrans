/**
 * 翻译页的状态与行为。
 *
 * 从 App.vue 里抽出来的原因:这个页面带着输入框、流式译文、方向读数、
 * 剪贴板、朗读,是三个页面里唯一有"实时状态机"的;混在 1100 行的
 * App.vue 里时,改动它总要连带读一遍设置页的代码。
 */
import { computed, nextTick, ref, watch } from "vue";
import { toast } from "vue-sonner";
import { api } from "@/api";
import { langLabel } from "@/types";
import type { Settings, StreamMsg } from "@/types";

/** 输入即译的防抖间隔 */
const AUTO_TRANSLATE_DEBOUNCE_MS = 800;

export function useTranslate(settings: () => Settings | null) {
  const source = ref("");
  const result = ref("");
  const errorMsg = ref("");
  /** start 事件里带出的方向/词典信息,例如「英语 → 中文(简体)」 */
  const langInfo = ref("");
  const streaming = ref(false);
  const speaking = ref(false);
  const copied = ref(false);
  const resultBox = ref<HTMLElement | null>(null);

  const sourceChars = computed(() => source.value.length);

  /** 未翻译时也显示将要使用的方向,让「当前的设置」始终可见 */
  const plannedDirection = computed(() => {
    const s = settings();
    if (!s) return "";
    if (s.transMode === "custom") {
      const src = s.sourceLang && s.sourceLang !== "auto" ? langLabel(s.sourceLang) : "自动";
      return `${src} → ${langLabel(s.targetLang)}`;
    }
    return "自动 → 中文(简体)";
  });

  const directionReadout = computed(() => langInfo.value || plannedDirection.value);

  function composeLangInfo(detected?: string, target?: string, dict?: boolean): string {
    if (dict) return "词典释义";
    const s = settings();
    if (!s) return "";
    if (s.transMode === "custom") {
      const src =
        s.sourceLang && s.sourceLang !== "auto" ? langLabel(s.sourceLang) : detected || "自动";
      return `${src} → ${langLabel(target || s.targetLang)}`;
    }
    return `${detected || "自动"} → 中文(简体)`;
  }

  function onStream(m: StreamMsg) {
    switch (m.type) {
      case "start":
        langInfo.value = composeLangInfo(m.detected, m.target, m.dict);
        break;
      case "delta":
        result.value += m.text ?? "";
        // 跟随滚动到底:长译文流式输出时用户不该手动往下拖
        nextTick(() => {
          if (resultBox.value) resultBox.value.scrollTop = resultBox.value.scrollHeight;
        });
        break;
      case "done":
        streaming.value = false;
        break;
      case "error":
        errorMsg.value = m.message ?? "未知错误";
        streaming.value = false;
        break;
      case "cancelled":
        streaming.value = false;
        break;
    }
  }

  async function translate() {
    const text = source.value.trim();
    if (!text) return;
    await cancel();
    result.value = "";
    errorMsg.value = "";
    langInfo.value = "";
    streaming.value = true;
    try {
      await api.translate(text, "main", onStream);
    } catch (e) {
      errorMsg.value = String(e);
    } finally {
      streaming.value = false;
    }
  }

  async function cancel() {
    if (streaming.value) await api.cancelTranslate("main");
  }

  function clearAll() {
    if (streaming.value) api.cancelTranslate("main");
    source.value = "";
    result.value = "";
    errorMsg.value = "";
    langInfo.value = "";
    streaming.value = false;
  }

  async function copyResult() {
    if (!result.value) return;
    await api.writeClipboard(result.value);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1400);
    toast.success("已复制译文");
  }

  async function speak() {
    const text = result.value || source.value;
    if (!text) return;
    if (speaking.value) {
      await api.ttsStop();
      speaking.value = false;
      return;
    }
    await api.ttsSpeak(text);
    speaking.value = true;
  }

  async function pasteFromClipboard() {
    const text = await api.readClipboard();
    if (text) source.value = text;
  }

  /* 输入即译:停止输入一段时间后自动翻译 */
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  watch(source, (v) => {
    if (!settings()?.autoTranslate) return;
    if (debounceTimer) clearTimeout(debounceTimer);
    if (!v.trim()) return;
    debounceTimer = setTimeout(() => translate(), AUTO_TRANSLATE_DEBOUNCE_MS);
  });

  return {
    source,
    result,
    errorMsg,
    langInfo,
    streaming,
    speaking,
    copied,
    resultBox,
    sourceChars,
    directionReadout,
    translate,
    cancel,
    clearAll,
    copyResult,
    speak,
    pasteFromClipboard,
  };
}
