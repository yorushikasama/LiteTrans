import { createApp } from "vue";
import Popup from "@/views/PopupView.vue";
import "@/assets/index.css";
import { installErrorBanner, installMockBadge } from "@/lib/errorBanner";
import { installTauriMock } from "@/lib/devMock";

if (installTauriMock()) installMockBadge();
installErrorBanner();
createApp(Popup).mount("#popup");
