import { createApp } from "vue";
import App from "./App.vue";
import "./assets/index.css";
import { installErrorBanner, installMockBadge } from "./errorBanner";
import { installTauriMock } from "./devMock";

if (installTauriMock()) installMockBadge();
installErrorBanner();
createApp(App).mount("#app");
