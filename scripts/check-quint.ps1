# Pin must match .github/workflows/ci.yml. Do not use @latest.
$QuintVersion = "0.32.0"
$Quint = "npx"
$Specs = Get-ChildItem -Path (Join-Path $PSScriptRoot "..\spec") -Filter "*.qnt"
if (-not $Specs) {
    Write-Error "no spec/*.qnt files"
    exit 1
}
foreach ($spec in $Specs) {
    Write-Host "== parse $($spec.Name)"
    & $Quint --yes "@informalsystems/quint@$QuintVersion" parse $spec.FullName
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "== typecheck $($spec.Name)"
    & $Quint --yes "@informalsystems/quint@$QuintVersion" typecheck $spec.FullName
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "== test $($spec.Name)"
    & $Quint --yes "@informalsystems/quint@$QuintVersion" test $spec.FullName
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host "== run $($spec.Name)"
    & $Quint --yes "@informalsystems/quint@$QuintVersion" run $spec.FullName --max-steps=30 --invariant=inv
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
Write-Host "quint ok"
