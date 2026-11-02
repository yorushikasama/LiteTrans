import { createApp } from "vue";
import Popup from "./Popup.vue";
import "./assets/index.css";
import { installErrorBanner, installMockBadge } from "./errorBanner";
import { installTauriMock } from "./devMock";

if (installTauriMock()) installMockBadge();
installErrorBanner();
createApp(Popup).mount("#popup");
