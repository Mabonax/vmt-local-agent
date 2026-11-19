param([ValidateSet('runtime','resume','rag','research','chat','json','scenario','craft')][string]$Action='runtime', [string]$Model='qwen3.5:4b')
$ErrorActionPreference='Stop'
$repo = Split-Path $PSScriptRoot -Parent
$checkData = Join-Path $repo '.local\verification-04\app-data'
New-Item -ItemType Directory -Path $checkData -Force | Out-Null
$priorAppData = $env:LOCALAPPDATA
Push-Location $repo
try {
    $env:LOCALAPPDATA = $checkData
    if ($Action -eq 'scenario') {
        $vswhere='C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
        $vsPath = & $vswhere -latest -products '*' -version '[17.0,18.0)' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        $devcmd = Join-Path $vsPath 'Common7\Tools\VsDevCmd.bat'
        $compilerEnvironment = & $env:ComSpec /d /s /c "`"$devcmd`" -no_logo -arch=x64 -host_arch=x64 && set"
        if ($LASTEXITCODE -ne 0) { throw 'Cannot initialize acceptance compiler' }
        foreach ($line in $compilerEnvironment) {
            $split=$line.IndexOf('=')
            if ($split -gt 0) { [Environment]::SetEnvironmentVariable($line.Substring(0,$split),$line.Substring($split+1),'Process') }
        }
        $env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH
        $env:LOCALAPPDATA = $checkData
    }
    & (Join-Path $repo 'target\release\vmt-agent-check.exe') "--$Action" $Model
    if ($LASTEXITCODE -ne 0) { throw "Acceptance $Action failed" }
} finally { $env:LOCALAPPDATA = $priorAppData; Pop-Location }
