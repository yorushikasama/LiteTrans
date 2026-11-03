# Accurately locate the popup window and sample its state.
# Shared by the drag tests below (dot-sourced).
# ASCII only: PowerShell 5.1 cannot parse UTF-8 Chinese without a BOM.

Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public class LtWin {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int idx);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);

    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L; public int T; public int R; public int B; }
    public delegate bool EnumProc(IntPtr h, IntPtr p);

    const int GWL_EXSTYLE = -20;
    const int WS_EX_TOPMOST = 0x8;
    public const uint LEFTDOWN = 0x2;
    public const uint LEFTUP = 0x4;

    static string T(IntPtr h) { var s = new StringBuilder(512); GetWindowTextW(h, s, 512); return s.ToString(); }

    // The popup is the only topmost, captionless LiteTrans window sized 420x330.
    // Matching on size + topmost is what distinguishes it from the main window,
    // which is also titled "LiteTrans" but is 815x637 and not topmost.
    public static IntPtr Popup(uint pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr pp) {
            uint wpid; GetWindowThreadProcessId(h, out wpid);
            if (wpid != pid) return true;
            if (T(h) != "LiteTrans") return true;
            if ((GetWindowLong(h, GWL_EXSTYLE) & WS_EX_TOPMOST) == 0) return true;
            RECT r; GetWindowRect(h, out r);
            int w = r.R - r.L, hh = r.B - r.T;
            // 420x330 CSS at 100% DPI, 630x495 at 150%
            if (!((w == 420 && hh == 330) || (w == 630 && hh == 495))) return true;
            found = h; return false;
        }, IntPtr.Zero);
        return found;
    }

    public static bool Visible(uint pid) {
        IntPtr h = Popup(pid);
        return h != IntPtr.Zero && IsWindowVisible(h);
    }

    public static string Sample(uint pid) {
        IntPtr h = Popup(pid);
        if (h == IntPtr.Zero) return "NO_POPUP";
        RECT r; GetWindowRect(h, out r);
        return String.Format("vis={0} rect=({1},{2})-({3},{4}) fgIsPopup={5}",
            IsWindowVisible(h), r.L, r.T, r.R, r.B, GetForegroundWindow() == h);
    }

    public static RECT Rect(uint pid) {
        RECT r; GetWindowRect(Popup(pid), out r); return r;
    }
}
"@

# --- helpers shared by the test steps ---
function Get-LtProcess {
    $p = Get-Process -Name litetrans -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $p) { throw "litetrans is not running" }
    return $p
}

function Wait-LtPopup([uint32]$processId, [int]$seconds = 30) {
    for ($i = 0; $i -lt ($seconds * 10); $i++) {
        if ([LtWin]::Visible($processId)) { return $true }
        Start-Sleep -Milliseconds 100
    }
    return $false
}
