// 截图翻译框选层:全屏遮罩 + 拖拽选框,纯 TS 无框架
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./assets/index.css";

const root = document.getElementById("capture")!;

// 遮罩(带巨大 box-shadow 挖洞效果由选框实现)+ 提示 + 选框
const overlay = document.createElement("div");
overlay.style.cssText =
  "position:fixed;inset:0;background:rgba(0,0,0,0.28);cursor:crosshair;user-select:none;z-index:10;";

const hint = document.createElement("div");
hint.textContent = "拖拽框选要翻译的区域 · Esc 取消";
hint.style.cssText =
  "position:fixed;top:24px;left:50%;transform:translateX(-50%);z-index:30;" +
  "background:rgba(15,23,42,0.85);color:#e2e8f0;font:13px 'Segoe UI','Microsoft YaHei',sans-serif;" +
  "padding:8px 18px;border-radius:8px;pointer-events:none;";

const sel = document.createElement("div");
sel.style.cssText =
  "position:fixed;border:1px solid #22d3ee;background:transparent;display:none;z-index:20;" +
  "box-shadow:0 0 0 100000px rgba(0,0,0,0.28);pointer-events:none;";

const sizeLabel = document.createElement("div");
sizeLabel.style.cssText =
  "position:fixed;display:none;z-index:30;background:#0ea5e9;color:#fff;" +
  "font:11px 'Segoe UI',sans-serif;padding:2px 8px;border-radius:4px;pointer-events:none;";

root.append(overlay, hint, sel, sizeLabel);

let sx = 0;
let sy = 0;
let dragging = false;

function reset() {
  dragging = false;
  sel.style.display = "none";
  sizeLabel.style.display = "none";
  overlay.style.background = "rgba(0,0,0,0.28)";
}

window.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return;
  dragging = true;
  sx = e.clientX;
  sy = e.clientY;
  overlay.style.background = "transparent";
});

window.addEventListener("mousemove", (e) => {
  if (!dragging) return;
  const x = Math.min(sx, e.clientX);
  const y = Math.min(sy, e.clientY);
  const w = Math.abs(e.clientX - sx);
  const h = Math.abs(e.clientY - sy);
  sel.style.cssText = `position:fixed;border:1px solid #22d3ee;display:block;z-index:20;box-shadow:0 0 0 100000px rgba(0,0,0,0.28);pointer-events:none;left:${x}px;top:${y}px;width:${w}px;height:${h}px;`;
  sizeLabel.style.cssText = `position:fixed;display:block;z-index:30;background:#0ea5e9;color:#fff;font:11px 'Segoe UI',sans-serif;padding:2px 8px;border-radius:4px;pointer-events:none;left:${x}px;top:${y + h + 6}px;`;
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
  invoke("ocr_translate", {
    x,
    y,
    w: Math.round(w * dpr),
    h: Math.round(h * dpr),
  }).catch(() => invoke("hide_capture"));
});

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") invoke("hide_capture");
});

listen("capture-reset", reset);
