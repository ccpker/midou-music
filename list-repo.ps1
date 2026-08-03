$r = Invoke-WebRequest -Uri 'https://api.github.com/repos/MakcRe/KuGouMusicApi/contents/module' -UseBasicParsing
$items = $r.Content | ConvertFrom-Json
foreach ($item in $items) {
    Write-Host $item.name
}
