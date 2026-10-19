# SPRT on Windows: current working copy ("new") vs a committed baseline ("base").
#
#   .\tools\sprt\run.ps1 v0.2 0 10
#   .\tools\sprt\run.ps1 -BaseRef v0.2 -Elo0 0 -Elo1 10 -TC 2+0.02 -Conc 8 -Rounds 300
#
# Needs fastchess.exe: put it on PATH or pass -Fastchess C:\path\to\fastchess.exe
# Freshly built .exe files can be refused for a short while (a scanner holds them
# until it has a verdict), so every executable is retried until it launches.
param(
  [Parameter(Mandatory = $true, Position = 0)][string]$BaseRef,
  [Parameter(Position = 1)][double]$Elo0 = 0,
  [Parameter(Position = 2)][double]$Elo1 = 10,
  [string]$TC = "2+0.02",
  [int]$Conc = 8,
  [int]$Rounds = 300,
  [string]$Fastchess = "fastchess"
)
$ErrorActionPreference = "Stop"

# w64devkit on PATH breaks Rust's GNU linking (gcc_eh); this window doesn't need it.
$env:Path = ($env:Path -split ';' | Where-Object { $_ -notlike '*w64devkit*' }) -join ';'
if ($env:Path -notlike '*\.cargo\bin*') { $env:Path += ";$HOME\.cargo\bin" }

$chess = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$book  = Join-Path $chess "tools\openings\book8.epd"
$bin   = Join-Path $chess "tools\sprt\bin"
$out   = Join-Path $chess "tools\sprt\out"
New-Item -ItemType Directory -Force $bin, $out | Out-Null
if (-not (Test-Path $book)) { throw "missing opening book: $book" }

function Wait-Runnable([string]$exe, [string[]]$testArgs) {
  for ($i = 0; $i -lt 60; $i++) {
    try { & $exe @testArgs | Out-Null; Write-Host "ready: $exe"; return }
    catch { Write-Host ("  waiting for scan: " + (Split-Path $exe -Leaf) + " (" + ($i * 5) + "s)"); Start-Sleep 5 }
  }
  throw "$exe still cannot be launched after 5 minutes"
}

Write-Host "== building new (working copy)"
Push-Location $chess
cargo build --release -p uci
if ($LASTEXITCODE -ne 0) { throw "build failed (new)" }
Copy-Item (Join-Path $chess "target\release\uci.exe") (Join-Path $bin "new.exe") -Force
Pop-Location

Write-Host "== building base ($BaseRef)"
$wt = Join-Path $env:TEMP "vanta_base"
if (Test-Path $wt) { git worktree remove --force $wt }
git worktree prune
git worktree add --detach $wt $BaseRef
if ($LASTEXITCODE -ne 0) { throw "cannot check out $BaseRef" }
$rel = (git -C $chess rev-parse --show-prefix).Trim()
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP "vanta_base_target"
Push-Location (Join-Path $wt $rel)
cargo build --release -p uci
$code = $LASTEXITCODE
Pop-Location
if ($code -ne 0) { throw "build failed (base)" }
Copy-Item (Join-Path $env:CARGO_TARGET_DIR "release\uci.exe") (Join-Path $bin "base.exe") -Force
Remove-Item Env:CARGO_TARGET_DIR
git worktree remove --force $wt

Write-Host "== waiting until the new executables can launch"
Wait-Runnable (Join-Path $bin "new.exe")  @("perft", "1")
Wait-Runnable (Join-Path $bin "base.exe") @("perft", "1")

Write-Host "== fastchess: new vs base, tc=$TC, sprt [$Elo0, $Elo1], concurrency $Conc"
$fcArgs = @(
  "-engine", "cmd=$bin\new.exe",  "name=new",
  "-engine", "cmd=$bin\base.exe", "name=base",
  "-each", "tc=$TC", "proto=uci", "option.Hash=16",
  "-openings", "file=$book", "format=epd", "order=random",
  "-rounds", "$Rounds", "-games", "2", "-repeat", "-concurrency", "$Conc",
  "-sprt", "elo0=$Elo0", "elo1=$Elo1", "alpha=0.05", "beta=0.05",
  "-pgnout", "file=$out\games.pgn",
  "-ratinginterval", "10"
)
& $Fastchess @fcArgs 2>&1 | Tee-Object -FilePath (Join-Path $out "run.log")