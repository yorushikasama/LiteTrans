import { createApp } from "vue";
import App from "@/App.vue";
import "@/assets/index.css";
import { installErrorBanner, installMockBadge } from "@/lib/errorBanner";
import { installTauriMock } from "@/lib/devMock";

if (installTauriMock()) installMockBadge();
installErrorBanner();
createApp(App).mount("#app");
