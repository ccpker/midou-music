$ErrorActionPreference = "Continue"

$TEST_HASH = "B3A52A7A8E2D4F6C1A9E3B5D7F2A4C8E6B1D3F5A"
$TEST_ALBUM_ID = "0"

$chars = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ"
$dfid = "-"
for ($i = 0; $i -lt 23; $i++) { $dfid += $chars[(Get-Random -Maximum 36)] }

$md5 = [System.Security.Cryptography.MD5]::Create()
$midBytes = $md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($dfid))
$mid = -join ($midBytes | ForEach-Object { $_.ToString("x2") })

$nowSec = [DateTimeOffset]::Now.ToUnixTimeSeconds()
$clienttime = "$nowSec"

$keySalt = "185672dd44712f60bb1736df5a377e82"
$keyInput = "${TEST_HASH}${keySalt}3116${mid}0"
$keyHash = -join ($md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($keyInput)) | ForEach-Object { $_.ToString("x2") })

$sigSalt = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA"
$pairs = @("area_code=1","behavior=play","clienttime=$clienttime","dfid=$dfid","mid=$mid","token=","userid=0","vip=0") | Sort-Object
$sigInput = "${sigSalt}$($pairs -join '')${sigSalt}"
$sigHash = -join ($md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($sigInput)) | ForEach-Object { $_.ToString("x2") })

Write-Host "dfid=$dfid" -ForegroundColor Gray
Write-Host "mid=$mid" -ForegroundColor Gray
Write-Host "key=$keyHash" -ForegroundColor Gray
Write-Host "sig=$sigHash" -ForegroundColor Gray

$headers = @{
    "User-Agent" = "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36"
    "Referer" = "https://www.kugou.com/"
    "dfid" = $dfid
    "clienttime" = $clienttime
    "mid" = $mid
}

# 标准版 v6 请求体
$bodyStd = @{
    area_code = "1"
    behavior = "play"
    qualities = @("128","320","flac")
    resource = @{
        album_audio_id = $TEST_ALBUM_ID
        collect_list_id = "3"
        collect_time = [DateTimeOffset]::Now.ToUnixTimeMilliseconds()
        hash = $TEST_HASH
        id = 0
        page_id = 1
        type = "audio"
    }
    token = ""
    tracker_param = @{
        all_m = 1
        auth = ""
        is_free_part = 0
        key = $keyHash
        module_id = 0
        need_climax = 1
        need_xcdn = 1
        open_time = ""
        pid = "411"
        pidversion = "3001"
        priv_vip_type = "6"
        viptoken = ""
    }
    userid = "0"
    vip = 0
}

$json = $bodyStd | ConvertTo-Json -Depth 10

Write-Host "`n--- v6 POST (raw body shown) ---" -ForegroundColor Cyan
Write-Host "Body: $($json.Substring(0, [Math]::Min(300, $json.Length)))" -ForegroundColor Gray

try {
    $r = Invoke-WebRequest -Uri "https://tracker.kugou.com/v6/priv_url" -Method POST -Headers $headers -Body $json -ContentType "application/json" -TimeoutSec 10
    Write-Host "`nHTTP Status: $($r.StatusCode)" -ForegroundColor Green
    Write-Host "Raw Content: $($r.Content)" -ForegroundColor Yellow
} catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Response: $($_.ErrorDetails | ConvertFrom-Json | ConvertTo-Json)" -ForegroundColor Gray
}

# 尝试不同 Content-Type (KuGou 用的是 Android 加密请求，可能不用 JSON)
Write-Host "`n--- v6 POST (x-www-form-urlencoded) ---" -ForegroundColor Cyan
try {
    $r2 = Invoke-WebRequest -Uri "https://tracker.kugou.com/v6/priv_url" -Method POST -Headers $headers -Body $json -ContentType "application/x-www-form-urlencoded" -TimeoutSec 10
    Write-Host "HTTP $($r2.StatusCode): $($r2.Content.Substring(0, [Math]::Min(300, $r2.Content.Length)))" -ForegroundColor Yellow
} catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}

# 尝试不加密的散装参数
Write-Host "`n--- v6 POST (散装参数) ---" -ForegroundColor Cyan
$flatParams = "area_code=1&behavior=play&hash=$TEST_HASH&album_audio_id=$TEST_ALBUM_ID&userid=0&dfid=$dfid&mid=$mid&key=$keyHash"
try {
    $r3 = Invoke-WebRequest -Uri "https://tracker.kugou.com/v6/priv_url" -Method POST -Headers $headers -Body $flatParams -ContentType "application/x-www-form-urlencoded" -TimeoutSec 10
    Write-Host "HTTP $($r3.StatusCode): $($r3.Content.Substring(0, [Math]::Min(300, $r3.Content.Length)))" -ForegroundColor Yellow
} catch {
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
}
