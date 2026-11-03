import { createApp } from "vue";
import App from "@/App.vue";
import "@/assets/index.css";
import { installErrorBanner, installMockBadge } from "@/lib/errorBanner";
import { bootstrapTheme } from "@/lib/theme";

installErrorBanner();

// 调试替身只在开发期需要:写成静态 import 的话,它和里面的示例数据会一起
// 进生产包(实测 228 kB),发布版白背这部分体积。动态 import 让构建器
// 在 PROD 分支下把它整块摇掉。
if (import.meta.env.DEV) {
  const { installTauriMock } = await import("@/lib/devMock");
  if (installTauriMock()) installMockBadge();
}

// 先定主题再挂载:否则深色用户会先闪一帧白底
await bootstrapTheme();
createApp(App).mount("#app");
