# Register existing native ArtCraft CLI MCP servers with VMT Local Agent.
# This script NEVER downloads, launches, or installs any executable.
# Only run after installing and verifying the CLIs from trusted ArtCraft releases.
[CmdletBinding()]
param(
    [switch]$IncludeUnconfined,
    [switch]$Force,
    [string]$WorkRoot = (Join-Path $HOME "craft-work-storytold")
)
$ErrorActionPreference = "Stop"
$bin = Join-Path $env:LOCALAPPDATA "Storytold\craft\bin"
$targetDir = Join-Path $env:LOCALAPPDATA "VMT\LocalAgent"
$target = Join-Path $targetDir "mcp-servers.json"
if ((Test-Path -LiteralPath $target) -and -not $Force) {
    throw "Registration already exists: $target. Review it or use -Force to overwrite."
}
$outRoot = Join-Path $WorkRoot "outputs"
if (-not (Test-Path -LiteralPath $WorkRoot -PathType Container)) {
    New-Item -ItemType Directory -Path $WorkRoot -Force | Out-Null
}
if (-not (Test-Path -LiteralPath $outRoot -PathType Container)) {
    New-Item -ItemType Directory -Path $outRoot -Force | Out-Null
}
$apps = @(
 "photocraft","vectorcraft","filmcraft","lightcraft","pdfcraft",
 "effectcraft","designcraft","gridcraft","wordcraft","deckcraft",
 "cadcraft","soundcraft","figcraft","materialcraft","texturecraft"
)
$unconfined = @("filmcraft","lightcraft","gridcraft","cadcraft")
$servers = @()
foreach ($app in $apps) {
    $exe = Join-Path $bin "$app-cli.exe"
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
        Write-Host "Not installed, skipped: $app"
        continue
    }
    if ($app -in $unconfined -and -not $IncludeUnconfined) {
        Write-Warning "$app has user-wide file access; skipped unless -IncludeUnconfined."
        continue
    }
    $arguments = @("mcp")
    switch ($app) {
        "vectorcraft" { $arguments += "--headless" }
        "lightcraft" { $arguments += "--compact" }
    }
    if ($app -notin $unconfined) {
        if ($app -eq "materialcraft") {
            $arguments += @("--read-root",$WorkRoot,"--write-root",$outRoot)
        } else {
            $arguments += @("--automation-read-root",$WorkRoot,
                "--automation-write-root",$outRoot)
        }
        if ($app -eq "pdfcraft") {
            $arguments += @("--root",$WorkRoot,"--compact")
        }
    }
    $servers += [ordered]@{
        id = $app
        command = (Resolve-Path -LiteralPath $exe).Path
        args = @($arguments)
    }
}
if ($servers.Count -eq 0) {
    throw "No suitable Craft CLI binaries found under $bin. Install the CLIs first."
}
New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
$payload = @{ servers = @($servers) } | ConvertTo-Json -Depth 7
$utf8 = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($target, $payload, $utf8)
Write-Host "Registered $($servers.Count) ArtCraft MCP servers at $target."
Write-Warning "Registration is not a sandbox. Every MCP process start/call requires approval in VMT."
