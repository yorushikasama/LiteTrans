# Popup interaction acceptance test.
#
# Verifies the three behaviours that must all hold at once, driving REAL mouse
# input through user32 (SendInput-class), because this bug cannot be reproduced
# with synthetic DOM events -- the hide came from a Windows focus event that
# only a real drag produces:
#
#   1. dragging the top bar MOVES the window and does NOT hide it
#   2. clicking the popup body does nothing (stays visible)
#   3. clicking outside the popup hides it
#
# Requires: the app running with the popup already shown.
# ASCII only: PowerShell 5.1 cannot parse UTF-8 Chinese without a BOM.

param(
    [string]$OutFile = "$env:TEMP\lt-popup-interaction.txt"
)

$ErrorActionPreference = "Stop"
. "$PSScriptRoot\popup-window.ps1"

function Log($m) { $m | Out-File -FilePath $OutFile -Append -Encoding ASCII }
"" | Out-File -FilePath $OutFile -Encoding ASCII

$proc = Get-LtProcess
$pid2 = [uint32]$proc.Id
Log "process pid=$pid2"

if (-not (Wait-LtPopup $pid2 5)) {
    Log "FAIL: popup is not visible; trigger a selection translate first"
    exit 2
}
Log ("start: " + [LtWin]::Sample($pid2))

$failures = 0

# ---- 1. body click must not hide ----
$r = [LtWin]::Rect($pid2)
$cx = [int]($r.L + ($r.R - $r.L) * 0.25)
$cy = [int]($r.T + ($r.B - $r.T) * 0.55)
Log "STEP 1: click body at ($cx,$cy)"
[void][LtWin]::SetCursorPos($cx, $cy)
Start-Sleep -Milliseconds 200
[LtWin]::mouse_event([LtWin]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 40
[LtWin]::mouse_event([LtWin]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 600
if ([LtWin]::Visible($pid2)) {
    Log "  PASS: still visible"
} else {
    Log "  FAIL: body click hid the popup"
    $failures++
    exit 3   # later steps need a visible popup
}

# ---- 2. drag by the top bar must move, not hide ----
$r = [LtWin]::Rect($pid2)
$L0 = $r.L; $T0 = $r.T
$gx = [int]($r.L + ($r.R - $r.L) * 0.5)
$gy = [int]($r.T + 20)          # inside the 32 CSS px top bar
Log "STEP 2: drag from ($gx,$gy)"
[void][LtWin]::SetCursorPos($gx, $gy)
Start-Sleep -Milliseconds 250
[LtWin]::mouse_event([LtWin]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 150
$hiddenMidDrag = $false
for ($i = 1; $i -le 12; $i++) {
    [void][LtWin]::SetCursorPos([int]($gx - 12 * $i), [int]($gy - 8 * $i))
    Start-Sleep -Milliseconds 60
    if (-not [LtWin]::Visible($pid2)) { $hiddenMidDrag = $true; break }
}
[LtWin]::mouse_event([LtWin]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 500

if ($hiddenMidDrag) {
    Log "  FAIL: popup disappeared mid-drag (focus loss while button held)"
    $failures++
} elseif (-not [LtWin]::Visible($pid2)) {
    Log "  FAIL: popup gone after drag"
    $failures++
} else {
    $r2 = [LtWin]::Rect($pid2)
    $dx = $r2.L - $L0; $dy = $r2.T - $T0
    Log ("  moved dx=$dx dy=$dy")
    if ([Math]::Abs($dx) -ge 40 -or [Math]::Abs($dy) -ge 40) {
        Log "  PASS: window moved and stayed visible"
    } else {
        Log "  FAIL: window did not move (drag region not working)"
        $failures++
    }
}

# ---- 3. outside click must hide ----
if ([LtWin]::Visible($pid2)) {
    $r = [LtWin]::Rect($pid2)
    $ox = [int]($r.R + 120)
    $oy = [int]($r.T + 80)
    $screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    if ($ox -gt $screen.Width - 20) { $ox = [int]($r.L - 120) }
    if ($ox -lt 20) { $ox = 40 }
    Log "STEP 3: click outside at ($ox,$oy)"
    [void][LtWin]::SetCursorPos($ox, $oy)
    Start-Sleep -Milliseconds 200
    [LtWin]::mouse_event([LtWin]::LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 60
    [LtWin]::mouse_event([LtWin]::LEFTUP, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 700
    if ([LtWin]::Visible($pid2)) {
        Log "  FAIL: outside click did not hide"
        $failures++
    } else {
        Log "  PASS: hidden"
    }
} else {
    Log "STEP 3: skipped (popup already gone)"
    $failures++
}

if ($failures -eq 0) {
    Log "RESULT: all interaction checks passed"
    exit 0
} else {
    Log "RESULT: $failures check(s) failed"
    exit 1
}
