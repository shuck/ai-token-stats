$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin;$env:USERPROFILE\mingw64\bin;" + $env:PATH
cargo build --release -p ai-token-stats
Copy-Item target\release\ai-token-stats.exe .\ai-token-stats.exe -Force
Write-Host "Built ai-token-stats.exe"
