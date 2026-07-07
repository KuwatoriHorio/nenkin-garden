# PreToolUse ガード（規約 §7・§11）: 人間所有ファイルの Edit/Write を物理的に拒否する。
# stdin から Claude Code のフック JSON を受け取り、tool_input.file_path を
# .claude/protected-paths.txt のパターンと照合。一致したら exit 2（＝ツール呼び出しをブロック）。
# トークンを消費せず確実にゲートするのが目的（規約本文の常駐に依存しない）。

$ErrorActionPreference = 'Stop'

try {
    $raw = [Console]::In.ReadToEnd()
} catch {
    exit 0
}
if ([string]::IsNullOrWhiteSpace($raw)) { exit 0 }

try {
    $j = $raw | ConvertFrom-Json
} catch {
    exit 0
}

$fp = $null
if ($j.tool_input -and $j.tool_input.file_path) { $fp = [string]$j.tool_input.file_path }
if ([string]::IsNullOrWhiteSpace($fp)) { exit 0 }

# パス正規化: バックスラッシュ→スラッシュ、末尾照合用に先頭スラッシュを保証。
$norm = ($fp -replace '\\', '/')
$normSlash = '/' + $norm.TrimStart('/')

$listFile = Join-Path $PSScriptRoot '..\protected-paths.txt'
if (-not (Test-Path $listFile)) { exit 0 }

$patterns = Get-Content -LiteralPath $listFile -Encoding UTF8 |
    ForEach-Object { $_.Trim() } |
    Where-Object { $_ -and -not $_.StartsWith('#') }

foreach ($p in $patterns) {
    $pat = '/' + ($p -replace '\\', '/').TrimStart('/')
    # 末尾一致: 正規化パスが "/<pattern>" で終わるか、パス区切り直後に現れる。
    if ($normSlash.EndsWith($pat) -or $normSlash -like "*$pat") {
        $leaf = Split-Path $fp -Leaf
        [Console]::Error.WriteLine(
            "BLOCKED (規約 §7/§11): '$leaf' は人間所有ファイルです（.claude/protected-paths.txt に登録）。" +
            "不変条件・ゴールデン・しきい値・規範文書の編集は自動 revert 対象。" +
            "変更が必要なら §8 に従い停止して人間の承認タスクで行ってください。"
        )
        exit 2
    }
}

exit 0
