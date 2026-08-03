$uri = 'https://api.github.com/search/repositories?q=echomusic+kugou&per_page=5'
$r = Invoke-WebRequest -Uri $uri -UseBasicParsing
$j = $r.Content | ConvertFrom-Json
foreach ($item in $j.items) {
    Write-Host "Name: $($item.full_name)"
    Write-Host "URL: $($item.html_url)"
    Write-Host ""
}
