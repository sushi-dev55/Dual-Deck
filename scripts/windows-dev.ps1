[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("cargo", "tauri")]
    [string]$Tool,

    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]]$ToolArguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-CargoTargetDirectory {
    $configuredTarget = $env:DUALDECK_CARGO_TARGET_DIR

    if ([string]::IsNullOrWhiteSpace($configuredTarget)) {
        if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
            throw "LOCALAPPDATA is unavailable. Set DUALDECK_CARGO_TARGET_DIR to an absolute path without spaces."
        }

        $configuredTarget = Join-Path $env:LOCALAPPDATA "DualDeck\cargo-target"
    }

    if (-not [System.IO.Path]::IsPathRooted($configuredTarget)) {
        throw "The Cargo target directory must be an absolute path: $configuredTarget"
    }

    $targetDirectory = [System.IO.Path]::GetFullPath($configuredTarget)

    if ($targetDirectory -match "\s") {
        throw "The Cargo target directory cannot contain whitespace because the Windows GNU linker cannot reliably process it: $targetDirectory"
    }

    $pathRoot = [System.IO.Path]::GetPathRoot($targetDirectory)
    if ($targetDirectory.TrimEnd("\", "/") -eq $pathRoot.TrimEnd("\", "/")) {
        throw "The Cargo target directory cannot be a drive root: $targetDirectory"
    }

    $directory = [System.IO.Directory]::CreateDirectory($targetDirectory)
    return $directory.FullName
}

function Test-GnuTargetRequested {
    foreach ($argument in $ToolArguments) {
        if ($argument -match "(?i)windows-gnu") {
            return $true
        }
    }

    return $false
}

function Test-DeveloperToolAvailable {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    return $null -ne (Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue)
}

function Add-MingwToolsToPath {
    $gnuTargetRequested = Test-GnuTargetRequested
    $gnuToolsMissing =
        -not (Test-DeveloperToolAvailable "gcc.exe") -or
        -not (Test-DeveloperToolAvailable "dlltool.exe")

    if (-not $gnuTargetRequested -and -not $gnuToolsMissing) {
        return
    }

    $configuredBin = $env:DUALDECK_MINGW_BIN
    if ([string]::IsNullOrWhiteSpace($configuredBin)) {
        $defaultBin = "C:\msys64\mingw64\bin"
        if (Test-Path -LiteralPath $defaultBin -PathType Container) {
            $configuredBin = $defaultBin
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($configuredBin)) {
        if (-not [System.IO.Path]::IsPathRooted($configuredBin)) {
            throw "DUALDECK_MINGW_BIN must be an absolute path: $configuredBin"
        }

        $mingwBin = [System.IO.Path]::GetFullPath($configuredBin)
        if ($mingwBin -match "\s") {
            throw "DUALDECK_MINGW_BIN cannot contain whitespace because GNU build tools may not process it reliably: $mingwBin"
        }

        if (-not (Test-Path -LiteralPath $mingwBin -PathType Container)) {
            throw "The MinGW bin directory does not exist: $mingwBin"
        }

        foreach ($executable in @("gcc.exe", "dlltool.exe")) {
            $executablePath = Join-Path $mingwBin $executable
            if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
                throw "The MinGW bin directory is missing $executable`: $mingwBin"
            }
        }

        $pathEntries = @($env:PATH -split [System.IO.Path]::PathSeparator)
        if (-not ($pathEntries | Where-Object { $_.TrimEnd("\") -ieq $mingwBin.TrimEnd("\") })) {
            $env:PATH = "$mingwBin$([System.IO.Path]::PathSeparator)$env:PATH"
        }
    }

    $missingExecutables = @(
        foreach ($executable in @("gcc.exe", "dlltool.exe")) {
            if (-not (Test-DeveloperToolAvailable $executable)) {
                $executable
            }
        }
    )

    if ($missingExecutables.Count -gt 0) {
        $missingList = $missingExecutables -join ", "
        throw "Required Windows GNU tools are unavailable: $missingList. Install the MSYS2 MinGW-w64 toolchain at C:\msys64\mingw64\bin or set DUALDECK_MINGW_BIN to its absolute, whitespace-free bin directory."
    }
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_TARGET_DIR = Get-CargoTargetDirectory
Add-MingwToolsToPath

switch ($Tool) {
    "cargo" {
        & cargo @ToolArguments
    }
    "tauri" {
        $tauriCommand = Join-Path $repositoryRoot "node_modules\.bin\tauri.cmd"
        if (-not (Test-Path -LiteralPath $tauriCommand -PathType Leaf)) {
            throw "The Tauri CLI is not installed. Run pnpm install --frozen-lockfile first."
        }

        & $tauriCommand @ToolArguments
    }
}

$processExitCode = Get-Variable -Name LASTEXITCODE -ValueOnly -ErrorAction SilentlyContinue
if ($null -eq $processExitCode) {
    exit 0
}

exit $processExitCode
