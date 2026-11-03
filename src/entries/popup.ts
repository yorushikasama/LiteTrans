import { createApp } from "vue";
import Popup from "@/views/PopupView.vue";
import "@/assets/index.css";
import { installErrorBanner, installMockBadge } from "@/lib/errorBanner";
import { bootstrapTheme } from "@/lib/theme";

installErrorBanner();

// 同 entries/main.ts:调试替身不进生产包
if (import.meta.env.DEV) {
  const { installTauriMock } = await import("@/lib/devMock");
  if (installTauriMock()) installMockBadge();
}

// 弹卡窗口也有自己的 WebView,同样要先定主题再挂载
await bootstrapTheme();
createApp(Popup).mount("#popup");
