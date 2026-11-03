/**
 * 主题解析与应用。
 *
 * 设置里存的是三态(system / light / dark),但 CSS 只认两种
 * (`data-theme="dark"` 有或没有)。这个模块负责把三态收敛成两种:
 *
 *   system → 读系统偏好,再写到 data-theme
 *   light  → data-theme="light"
 *   dark   → data-theme="dark"
 *
 * 把"跟随系统"解析在前端做,好处是 CSS 里的深色变量只需要写一份 ——
 * 否则 @media 和 [data-theme=dark] 两套选择器得各抄一整段色值,
 * 以后改色必漏一处。
 */
import { ref } from "vue";
import { api } from "@/api";
import type { Theme } from "@/types";

const MEDIA = "(prefers-color-scheme: dark)";

/**
 * 实际生效的深浅,供 Sonner 这类"自己判断主题"的第三方组件使用。
 * 它们默认看 matchMedia,但用户可能强制了浅色/深色,那时会跟界面反着来。
 */
export const resolvedTheme = ref<"light" | "dark">("light");

/** 当前实际生效的是不是深色 */
function systemPrefersDark(): boolean {
  return typeof matchMedia === "function" && matchMedia(MEDIA).matches;
}

/** 把设置值换算成要写到 <html> 上的 data-theme */
export function resolveTheme(theme: Theme): "light" | "dark" {
  if (theme === "dark") return "dark";
  if (theme === "light") return "light";
  return systemPrefersDark() ? "dark" : "light";
}

/**
 * 应用主题。立即生效,无需重启。
 * 返回取消订阅函数:只有 system 时才有意义(系统切换时跟着变)。
 */
export function applyTheme(theme: Theme): () => void {
  const root = document.documentElement;
  const write = () => {
    const next = resolveTheme(theme);
    root.dataset.theme = next;
    resolvedTheme.value = next;
  };
  write();

  if (theme !== "system" || typeof matchMedia !== "function") {
    return () => {};
  }
  // 跟随系统时,Windows 切换深浅色要实时反映过来
  const mq = matchMedia(MEDIA);
  mq.addEventListener("change", write);
  return () => mq.removeEventListener("change", write);
}

/** 同上,但先退订上一次的监听 —— 反复改主题不该让 matchMedia 监听堆积 */
let dispose: (() => void) | null = null;
export function useTheme(theme: Theme): void {
  dispose?.();
  dispose = applyTheme(theme);
}

/**
 * 启动时套用一次。
 *
 * 必须在 mount 之前 await:默认(:root)是浅色,如果等页面渲染完再套深色,
 * 用户会先看到一帧白底闪一下。多花一次 IPC(毫秒级)换掉这个闪烁。
 */
export async function bootstrapTheme(): Promise<void> {
  try {
    const s = await api.getSettings();
    useTheme(s.theme ?? "system");
  } catch {
    /* 浏览器直接打开页面时没有后端,保持默认浅色 */
  }
}
