// 截图翻译框选层:全屏遮罩 + 拖拽选框,纯 TS 无框架
// 配色跟随五音主题:选框用宫色,尺寸读数用等宽字
import { api } from "@/api";
import { listen } from "@tauri-apps/api/event";
import "@/assets/index.css";

const root = document.getElementById("capture")!;

const overlay = document.createElement("div");
overlay.className = "lt-cap-overlay";

const hint = document.createElement("div");
hint.className = "lt-cap-hint";
hint.innerHTML =
  '<span class="lt-cap-dot"></span>拖拽框选要翻译的区域<span class="lt-cap-key">Esc</span>取消';

const sel = document.createElement("div");
sel.className = "lt-cap-sel";

const sizeLabel = document.createElement("div");
sizeLabel.className = "lt-cap-size lt-num";

root.append(overlay, hint, sel, sizeLabel);

let sx = 0;
let sy = 0;
let dragging = false;

function reset() {
  dragging = false;
  sel.style.display = "none";
  sizeLabel.style.display = "none";
  overlay.style.display = "block";
  hint.style.display = "flex";
  overlay.style.opacity = "1";
}

window.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return;
  dragging = true;
  sx = e.clientX;
  sy = e.clientY;
  overlay.style.opacity = "0";
});

window.addEventListener("mousemove", (e) => {
  if (!dragging) return;
  const x = Math.min(sx, e.clientX);
  const y = Math.min(sy, e.clientY);
  const w = Math.abs(e.clientX - sx);
  const h = Math.abs(e.clientY - sy);
  Object.assign(sel.style, {
    display: "block",
    left: `${x}px`,
    top: `${y}px`,
    width: `${w}px`,
    height: `${h}px`,
  });
  Object.assign(sizeLabel.style, {
    display: "block",
    left: `${x}px`,
    top: `${y + h + 6}px`,
  });
  sizeLabel.textContent = `${Math.round(w)} × ${Math.round(h)}`;
});

window.addEventListener("mouseup", (e) => {
  if (!dragging) return;
  dragging = false;
  const w = Math.abs(e.clientX - sx);
  const h = Math.abs(e.clientY - sy);
  if (w < 10 || h < 10) {
    reset();
    return;
  }
  const dpr = window.devicePixelRatio || 1;
  const x = Math.round(Math.min(sx, e.clientX) * dpr);
  const y = Math.round(Math.min(sy, e.clientY) * dpr);
  // 框选完成后立刻整体隐藏,避免遮罩污染截图(Rust 端延迟 180ms 抓屏)
  overlay.style.display = "none";
  sel.style.display = "none";
  sizeLabel.style.display = "none";
  hint.style.display = "none";
  api
    .ocrTranslate({
      x,
      y,
      w: Math.round(w * dpr),
      h: Math.round(h * dpr),
    })
    .catch(() => api.hideCapture());
});

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") api.hideCapture();
});

listen("capture-reset", reset);
