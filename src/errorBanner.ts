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
  window.addEventListener("unhandledrejection", (e) =>
    show(`Promise 拒绝: ${e.reason}`),
  );
}
