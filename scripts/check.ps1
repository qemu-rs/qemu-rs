Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Set-Variable -Name PSNativeCommandUseErrorActionPreference -Value $true -Scope Global -ErrorAction SilentlyContinue

$ScriptDir = $PSScriptRoot
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir '..')).Path

$checkPaths = @(
    (Join-Path $RepoRoot 'qemu-plugin'),
    (Join-Path $RepoRoot 'qemu-plugin-sys')
    # (Join-Path $RepoRoot 'plugins/icount'),
    # (Join-Path $RepoRoot 'plugins/tiny'),
    # (Join-Path $RepoRoot 'plugins/tiny-system'),
    # (Join-Path $RepoRoot 'plugins/tracer')
)

Push-Location $RepoRoot
try {
    & cargo fmt --all --check -v
} finally {
    Pop-Location
}

foreach ($checkPath in $checkPaths) {
    $manifestPath = Join-Path $checkPath 'Cargo.toml'

    & cargo +nightly hack --manifest-path $manifestPath `
        --mutually-exclusive-features=plugin-api-v0,plugin-api-v1,plugin-api-v2,plugin-api-v3,plugin-api-v4,plugin-api-v5,plugin-api-v6 `
        --at-least-one-of=plugin-api-v0,plugin-api-v1,plugin-api-v2,plugin-api-v3,plugin-api-v4,plugin-api-v5 `
        --feature-powerset `
        --exclude-no-default-features `
        check --lib -vv

    & cargo +nightly hack --manifest-path $manifestPath `
        --mutually-exclusive-features=plugin-api-v0,plugin-api-v1,plugin-api-v2,plugin-api-v3,plugin-api-v4,plugin-api-v5,plugin-api-v6 `
        --at-least-one-of=plugin-api-v0,plugin-api-v1,plugin-api-v2,plugin-api-v3,plugin-api-v4,plugin-api-v5 `
        --feature-powerset `
        --exclude-no-default-features `
        clippy --lib -vv
}
