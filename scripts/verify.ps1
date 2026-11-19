# Zero-friction verification: locate Cargo, pick a working linker, run the suite.
$ErrorActionPreference = 'Stop'

$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) {
    $fallback = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (Test-Path $fallback) {
        $cargo = $fallback
        $env:PATH = (Split-Path $cargo) + ';' + $env:PATH
    }
}
if (-not $cargo) {
    Write-Error 'Cargo not found. Install Rust 1.70+ from https://rustup.rs'
    exit 2
}

$toolchain = ''
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    # No MSVC linker: use the GNU toolchain and a WinLibs MinGW bin directory.
    $roots = Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages') -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like '*WinLibs*' }
    foreach ($root in $roots) {
        $bin = Join-Path $root.FullName 'mingw64\bin'
        if (Test-Path (Join-Path $bin 'gcc.exe')) {
            $env:PATH = "$bin;$env:PATH"
            break
        }
    }
    $rustup = Join-Path (Split-Path $cargo) 'rustup.exe'
    if ((Test-Path $rustup) -and (& $rustup toolchain list | Select-String 'windows-gnu')) {
        $toolchain = '+stable-x86_64-pc-windows-gnu'
    }
}

Write-Host "Using $cargo $toolchain"
if ($toolchain) {
    & $cargo $toolchain test
} else {
    & $cargo test
}
exit $LASTEXITCODE
