#!/usr/bin/env bash
# 打包便携(绿色)版:单目录解压即用,配置跟着程序走
#
# 与安装版共用同一个 litetrans.exe,区别只在旁边有没有 portable.txt:
# 该文件存在时 paths.rs 把数据目录判定为 <exe目录>/data。
# 所以这里除了 exe 之外必须放进那个标记文件,否则解压出来仍是安装版行为。
#
# 用法: bash scripts/make-portable.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="$ROOT/src-tauri/target/release/litetrans.exe"
OUT="$ROOT/release"
NAME="LiteTrans_0.1.0_x64_portable"
STAGE="$OUT/$NAME"

if [ ! -f "$EXE" ]; then
  echo "找不到 $EXE" >&2
  echo "请先执行:npm run tauri build" >&2
  exit 1
fi

rm -rf "$STAGE" "$OUT/$NAME.zip"
mkdir -p "$STAGE"

cp "$EXE" "$STAGE/LiteTrans.exe"

# 便携标记:没有它就不是便携版
printf 'portable\r\n' > "$STAGE/portable.txt"

cat > "$STAGE/使用说明.txt" <<'TXT'
LiteTrans 便携版
================

怎么用
------
1. 把整个文件夹解压到任意位置(桌面、U盘、网盘同步目录都行)
2. 双击 LiteTrans.exe 运行,首次会自动创建 data 子目录
3. 程序常驻托盘;在别的窗口选中文字后按 Alt+Q 划词翻译

便携体现在哪
------------
配置(含模型档案、热键、翻译方向)和历史记录都存在本文件夹的 data\ 目录,
不写 %APPDATA%,不写注册表(因此没有"开机自启"这一项)。
想让配置跟着走,把整个文件夹一起拷走即可。

注意
----
- API Key 保存在 Windows 凭据管理器,不随文件夹迁移;换机器需要重新填一次
- 划词依赖模拟 Ctrl+C:某些终端、游戏里不生效,这些场景请用 Alt+X 截图翻译
- 若目标应用以管理员权限运行,本程序也需要以管理员身份运行才能划词

卸载
----
直接删除整个文件夹。想要更干净,再到"凭据管理器 → Windows 凭据"里
删除以 LiteTrans 开头的条目。
TXT

# 打 zip:用 PowerShell 的 Compress-Archive,避免依赖 zip 命令
powershell -NoProfile -Command "
Compress-Archive -Path '$(cygpath -w "$STAGE")\*' -DestinationPath '$(cygpath -w "$OUT/$NAME.zip")' -Force
" >/dev/null

SIZE=$(stat -c%s "$OUT/$NAME.zip" 2>/dev/null || stat -f%z "$OUT/$NAME.zip")
echo "便携版已生成:"
echo "  目录 $STAGE"
echo "  zip  $OUT/$NAME.zip ($((SIZE / 1048576)) MiB)"