import { createApp } from "vue";
import Popup from "./Popup.vue";
import "./assets/index.css";
import { installErrorBanner } from "./errorBanner";
import { installTauriMock } from "./devMock";

installTauriMock();
installErrorBanner();
createApp(Popup).mount("#popup");
