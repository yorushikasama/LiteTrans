// 全局错误可视化:任何未捕获错误/Promise 拒绝都在页面顶部显示红条
// 方便开发期第一时间发现页面 JS 崩溃(如模块求值失败导致整页假死)
export function installErrorBanner() {
  const show = (msg: string) => {
    let bar = document.getElementById("lt-error-banner");
    if (!bar) {
      bar = document.createElement("div");
      bar.id = "lt-error-banner";
      bar.style.cssText =
        "position:fixed;top:0;left:0;right:0;z-index:2147483647;background:#d84b4b;color:#fff;" +
        "font:12px/1.5 'Segoe UI',sans-serif;padding:6px 12px;white-space:pre-wrap;max-height:40vh;overflow:auto;" +
        "pointer-events:none;"; // 仅展示,绝不拦截点击
      document.body.appendChild(bar);
    }
    bar.textContent = `⚠ ${msg}`;
  };
  window.addEventListener("error", (e) => show(e.message || "未知错误"));
  window.addEventListener("unhandledrejection", (e) => show(`Promise 拒绝: ${e.reason}`));
}

/**
 * 浏览器调试态标识。
 *
 * 在普通浏览器里打开 devUrl 时,后端由 devMock 顶替:没有真实划词、
 * 没有真实模型。此前界面和正式版完全一样,导致"选中文字没翻译"之类
 * 的误判 —— 看到的是 mock 数据,却以为应用坏了。这里在页面底部挂一条
 * 常驻提示,并且把标题改掉,一眼可辨。
 */
export function installMockBadge() {
  const badge = document.createElement("div");
  badge.id = "lt-mock-badge";
  badge.textContent = "浏览器调试 · 译文为 mock";
  badge.title = "浏览器调试模式:无真实划词与翻译,译文由 mock 生成";
  // 靠右下角收成一枚小胶囊,而不是横贯底部的整条:整条会盖住操作条最下面的
  // 22px,把「翻译/粘贴/复制」按钮压掉一半,调试时反而点不到按钮。
  badge.style.cssText =
    "position:fixed;right:8px;bottom:8px;z-index:2147483646;max-width:60vw;" +
    "background:#8b5500;color:#ffe9bf;font:11px/1.5 'Segoe UI','Microsoft YaHei',sans-serif;" +
    "padding:3px 9px;border-radius:999px;letter-spacing:0.02em;pointer-events:none;" +
    "box-shadow:0 2px 8px rgb(0 0 0 / 0.25);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;";
  document.body.appendChild(badge);
  document.title = `[调试] ${document.title}`;
}
