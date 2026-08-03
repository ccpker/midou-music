$ErrorActionPreference = "Continue"
$TEST_HASH = "B3A52A7A8E2D4F6C1A9E3B5D7F2A4C8E6B1D3F5A"
$TEST_ALBUM_ID = "0"

# 生成随机 dfid
$chars = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ"
$dfid = "-"
for ($i = 0; $i -lt 23; $i++) { $dfid += $chars[(Get-Random -Maximum 36)] }

$md5 = [System.Security.Cryptography.MD5]::Create()
$midBytes = $md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($dfid))
$mid = -join ($midBytes | ForEach-Object { $_.ToString("x2") })

$nowSec = [DateTimeOffset]::Now.ToUnixTimeSeconds()
$clienttime = "$nowSec"

# 标准版 key (非lite)
$keySalt = "185672dd44712f60bb1736df5a377e82"
$keyInput = "${TEST_HASH}${keySalt}3116${mid}0"
$keyBytes = $md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($keyInput))
$keyHash = -join ($keyBytes | ForEach-Object { $_.ToString("x2") })

$sigSalt = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA"
$pairs = @("area_code=1","behavior=play","clienttime=$clienttime","dfid=$dfid","mid=$mid","token=","userid=0","vip=0") | Sort-Object
$sigInput = "${sigSalt}$($pairs -join '')${sigSalt}"
$sigBytes = $md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($sigInput))
$sigHash = -join ($sigBytes | ForEach-Object { $_.ToString("x2") })

$headers = @{
    "User-Agent" = "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36"
    "Referer" = "https://www.kugou.com/"
    "dfid" = $dfid
    "clienttime" = $clienttime
    "mid" = $mid
}

$body = @{
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
} | ConvertTo-Json -Depth 10

Write-Host "dfid=$dfid" -ForegroundColor Gray
Write-Host "mid=$($mid.Substring(0,8))..." -ForegroundColor Gray
Write-Host "key=$($keyHash.Substring(0,8))..." -ForegroundColor Gray
Write-Host "sig=$($sigHash.Substring(0,8))..." -ForegroundColor Gray

Write-Host "`n--- 测试1: tracker.kugou.com HTTP连通性 ---" -ForegroundColor Cyan
try {
    $r = Invoke-WebRequest -Uri "https://tracker.kugou.com/" -Method GET -TimeoutSec 5
    Write-Host "✅ HTTP $($r.StatusCode)" -ForegroundColor Green
} catch {
    Write-Host "❌ $($_ | Out-String)" -ForegroundColor Red
}

Write-Host "`n--- 测试2: v6 (appid in header) ---" -ForegroundColor Cyan
$headers2 = @{
    "User-Agent" = "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36"
    "Referer" = "https://www.kugou.com/"
    "dfid" = $dfid
    "clienttime" = $clienttime
    "mid" = $mid
    "appid" = "3116"
}
try {
    $r2 = Invoke-WebRequest -Uri "https://tracker.kugou.com/v6/priv_url" -Method POST -Headers $headers2 -Body $body -ContentType "application/json" -TimeoutSec 10
    $d2 = $r2.Content | ConvertFrom-Json
    Write-Host "HTTP $($r2.StatusCode): errcode=$($d2.errcode) msg=$($d2.error)" -ForegroundColor Gray
    if ($d2.errcode -eq 0) { Write-Host "✅ 成功!" -ForegroundColor Green } else { Write-Host "❌ 失败" -ForegroundColor Red }
} catch { Write-Host "❌ $($_.Exception.Message)" -ForegroundColor Red }

Write-Host "`n--- 测试3: v6 (appid in query) ---" -ForegroundColor Cyan
try {
    $r3 = Invoke-WebRequest -Uri "https://tracker.kugou.com/v6/priv_url?dfid=$dfid&mid=$mid&appid=3116&clienttime=$clienttime&signature=$sigHash" -Method POST -Headers $headers -Body $body -ContentType "application/json" -TimeoutSec 10
    $d3 = $r3.Content | ConvertFrom-Json
    Write-Host "HTTP $($r3.StatusCode): errcode=$($d3.errcode) msg=$($d3.error)" -ForegroundColor Gray
    if ($d3.errcode -eq 0) { Write-Host "✅ 成功!" -ForegroundColor Green } else { Write-Host "❌ 失败" -ForegroundColor Red }
} catch { Write-Host "❌ $($_.Exception.Message)" -ForegroundColor Red }

Write-Host "`n--- 测试4: v6 (标准版 appid=1005) ---" -ForegroundColor Cyan
# 标准版 signKey 用不同盐值
$keySaltStd = "57ae12eb6890223e355ccfcb74edf70d"
$keyInputStd = "${TEST_HASH}${keySaltStd}1005${mid}0"
$keyBytesStd = $md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($keyInputStd))
$keyHashStd = -join ($keyBytesStd | ForEach-Object { $_.ToString("x2") })

$sigSaltStd = "OIlwieks28dk2k092lksi2UIkp"
$pairsStd = @("area_code=1","behavior=play","clienttime=$clienttime","dfid=$dfid","mid=$mid","token=","userid=0","vip=0") | Sort-Object
$sigInputStd = "${sigSaltStd}$($pairsStd -join '')${sigSaltStd}"
$sigBytesStd = $md5.ComputeHash([Text.Encoding]::UTF8.GetBytes($sigInputStd))
$sigHashStd = -join ($sigBytesStd | ForEach-Object { $_.ToString("x2") })

$headers4 = @{
    "User-Agent" = "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36"
    "Referer" = "https://www.kugou.com/"
    "dfid" = $dfid
    "clienttime" = $clienttime
    "mid" = $mid
    "appid" = "1005"
}
$body4 = $body.Clone()
$body4.tracker_param.key = $keyHashStd
try {
    $r4 = Invoke-WebRequest -Uri "https://tracker.kugou.com/v6/priv_url" -Method POST -Headers $headers4 -Body ($body4 | ConvertTo-Json -Depth 10) -ContentType "application/json" -TimeoutSec 10
    $d4 = $r4.Content | ConvertFrom-Json
    Write-Host "HTTP $($r4.StatusCode): errcode=$($d4.errcode) msg=$($d4.error)" -ForegroundColor Gray
    if ($d4.errcode -eq 0) { Write-Host "✅ 成功! URL长度=$($d4.data.url.Length)" -ForegroundColor Green } else { Write-Host "❌ 失败" -ForegroundColor Red }
} catch { Write-Host "❌ $($_.Exception.Message)" -ForegroundColor Red }
