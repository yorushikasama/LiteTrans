import { createApp } from "vue";
import Popup from "./Popup.vue";
import "./assets/index.css";
import { installErrorBanner } from "./errorBanner";

installErrorBanner();
createApp(Popup).mount("#popup");
