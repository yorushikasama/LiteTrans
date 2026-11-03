# Bring the popup on screen without a human: put text in a real editor,
# select it, press the global hotkey. This exercises the REAL capture path
# (Ctrl+C injection included), so a successful run means selection capture
# works too -- not just that the window can be shown.
#
# ASCII only: PowerShell 5.1 cannot parse UTF-8 Chinese without a BOM.

param(
    [string]$Text = "Simplicity is the ultimate sophistication.",
    [int]$WaitSeconds = 20
)

$ErrorActionPreference = "Stop"
. "$PSScriptRoot\popup-window.ps1"

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public class LtKey {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int n);

    public delegate bool EnumProc(IntPtr h, IntPtr p);

    const byte VK_MENU = 0x12;
    const byte VK_Q = 0x51;
    const uint KEYUP = 0x0002;

    static string T(IntPtr h) { var s = new StringBuilder(512); GetWindowTextW(h, s, 512); return s.ToString(); }
    static string C(IntPtr h) { var s = new StringBuilder(512); GetClassNameW(h, s, 512); return s.ToString(); }

    public static void ForceForeground(IntPtr h) {
        uint dummy1, dummy2;
        uint fg = GetWindowThreadProcessId(GetForegroundWindow(), out dummy1);
        uint me = GetCurrentThreadId();
        uint target = GetWindowThreadProcessId(h, out dummy2);
        AttachThreadInput(me, fg, true);
        AttachThreadInput(me, target, true);
        ShowWindow(h, 9);
        SetForegroundWindow(h);
        AttachThreadInput(me, target, false);
        AttachThreadInput(me, fg, false);
    }

    // first visible top-level window of the process with a title
    public static IntPtr AppWindow(uint pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr p) {
            uint wpid; GetWindowThreadProcessId(h, out wpid);
            if (wpid != pid) return true;
            if (!IsWindowVisible(h)) return true;
            string c = C(h);
            if (c == "ConsoleWindowClass" || c == "ApplicationFrameWindow") return true;
            if (T(h).Length == 0) return true;
            found = h; return false;
        }, IntPtr.Zero);
        return found;
    }

    public static void AltQ() {
        keybd_event(VK_MENU, 0, 0, IntPtr.Zero);
        System.Threading.Thread.Sleep(60);
        keybd_event(VK_Q, 0, 0, IntPtr.Zero);
        System.Threading.Thread.Sleep(60);
        keybd_event(VK_Q, 0, KEYUP, IntPtr.Zero);
        System.Threading.Thread.Sleep(60);
        keybd_event(VK_MENU, 0, KEYUP, IntPtr.Zero);
    }
}
"@

# 1. need a text editor with a selection
$editor = Get-Process -Name notepad -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $editor) {
    cmd /c start "" notepad.exe | Out-Null
    Start-Sleep -Milliseconds 2500
    $editor = Get-Process -Name notepad -ErrorAction SilentlyContinue | Select-Object -First 1
}
if (-not $editor) { throw "no text editor available for the selection test" }

$h = [LtKey]::AppWindow([uint32]$editor.Id)
if ($h -eq [IntPtr]::Zero) { throw "editor window not found" }
[LtKey]::ForceForeground($h)
Start-Sleep -Milliseconds 600

# 2. type the text and select it all
[System.Windows.Forms.SendKeys]::SendWait("^a")
[System.Windows.Forms.SendKeys]::SendWait("{DEL}")
[System.Windows.Forms.SendKeys]::SendWait($Text)
Start-Sleep -Milliseconds 400
[System.Windows.Forms.SendKeys]::SendWait("^a")
Start-Sleep -Milliseconds 400

# 3. fire the global hotkey
[LtKey]::AltQ()

$app = Get-LtProcess
$pid2 = [uint32]$app.Id
$shown = Wait-LtPopup $pid2 $WaitSeconds
if (-not $shown) {
    throw "popup did not appear within $WaitSeconds s (hotkey or capture failed)"
}
Write-Output ("popup shown: " + [LtWin]::Sample($pid2))
exit 0
