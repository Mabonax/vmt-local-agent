param([ValidateSet('check','build','test','clippy','run')][string]$Action='build', [switch]$Release, [string]$Binary='', [switch]$Library, [string[]]$CargoArguments=@())
$ErrorActionPreference='Stop'
$vswhere='C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$installation = & $vswhere -latest -products '*' -version '[17.0,18.0)' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $installation) { throw 'Visual Studio 2022 C++ tools are required.' }
$devcmd=Join-Path $installation 'Common7\Tools\VsDevCmd.bat'
# Repeated checks in one shell must not repeatedly append VS paths until cmd.exe
# exceeds its environment/command limits. Reuse only the selected VS 2022 x64 env.
$alreadyInitialized = $env:VSINSTALLDIR -and
    ($env:VSINSTALLDIR.TrimEnd('\') -eq $installation.TrimEnd('\')) -and
    ($env:VSCMD_ARG_TGT_ARCH -eq 'x64') -and ($env:VSCMD_VER -like '17.*')
if (-not $alreadyInitialized) {
    # Capture the environment privately; never print the resulting variables.
    $compilerEnvironment = & $env:ComSpec /d /s /c "`"$devcmd`" -no_logo -arch=x64 -host_arch=x64 && set"
    if ($LASTEXITCODE -ne 0) { throw 'Could not initialize the C++ toolchain.' }
    foreach($line in $compilerEnvironment) {
        $split=$line.IndexOf('=')
        if($split -gt 0) { [Environment]::SetEnvironmentVariable($line.Substring(0,$split),$line.Substring($split+1),'Process') }
    }
}
$cargo=Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$arguments=@($Action,'--locked')
if($Binary) { $arguments+=@('--bin',$Binary) }
if($Library) { $arguments+='--lib' }
if($Release -and $Action -in @('build','run')) { $arguments+='--release' }
$arguments+=$CargoArguments
Push-Location $PSScriptRoot
try { & $cargo @arguments; if($LASTEXITCODE -ne 0){ throw "cargo $Action failed" } } finally { Pop-Location }
