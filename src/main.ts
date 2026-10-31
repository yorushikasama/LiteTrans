import { createApp } from "vue";
import App from "./App.vue";
import "./assets/index.css";
import { installErrorBanner } from "./errorBanner";
import { installTauriMock } from "./devMock";

installTauriMock();
installErrorBanner();
createApp(App).mount("#app");
