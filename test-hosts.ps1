$hosts = @(
    'https://api.pzer.ml',
    'https://api.tikui.top',
    'https://kugou-api-pi.vercel.app',
    'https://kugou-api.zeabur.app',
    'https://kugou-api.railway.app',
    'https://api.lyceum.cn'
)
foreach ($h in $hosts) {
    try {
        $r = Invoke-WebRequest -Uri $h -TimeoutSec 5 -UseBasicParsing
        Write-Host "$h : $($r.StatusCode)"
    } catch {
        Write-Host "$h : $($_.Exception.Message)"
    }
}
