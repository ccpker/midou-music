# test-kugou-v6.ps1 - 验证酷狗 v6 播放 API
# 用法: 直接运行（独立测试，不依赖 Tauri）
$ErrorActionPreference = "Continue"

# 测试1: 搜索
Write-Host "`n=== 测试1: 酷狗搜索 ===" -ForegroundColor Cyan
$searchUrl = "https://songsearch.kugou.com/song_search_v2"
$params = @{
    keyword = "周杰伦"
    platform = "WebFilter"
    format = "json"
    page = "1"
    pagesize = "5"
    userid = "-1"
}
try {
    $resp = Invoke-RestMethod -Uri $searchUrl -Method GET -Headers @{
        "User-Agent" = "Mozilla/5.0"
        "Referer" = "https://www.kugou.com/"
    } -Body $params
    if ($resp.error_code -eq 0) {
        $count = $resp.data.lists.Count
        Write-Host "✅ 搜索成功，找到 $($count) 首" -ForegroundColor Green
        $first = $resp.data.lists[0]
        $hash = $first.FileHash
        $name = $first.FileName
        Write-Host "  示例: $name (hash=$($hash.Substring(0,8))...)"
        $script:TEST_HASH = $hash
        $script:TEST_NAME = $name
        $script:TEST_ALBUM_ID = $first.AlbumId
    } else {
        Write-Host "❌ 搜索失败: $($resp.error_code)" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ 请求失败: $_" -ForegroundColor Red
}

# 测试2: v6 播放地址（无token）
if ($TEST_HASH) {
    Write-Host "`n=== 测试2: v6播放URL（无token） ===" -ForegroundColor Cyan
    $v6Url = "https://tracker.kugou.com/v6/priv_url"

    # 生成随机 dfid
    $chars = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    $dfid = "-"
    for ($i = 0; $i -lt 23; $i++) {
        $dfid += $chars[(Get-Random -Maximum 36)]
    }

    $mid = -join ([System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($dfid)) | ForEach-Object { $_.ToString("x2") })
    $nowSec = [int64](Get-Date -UnixTimeSeconds)
    $clienttime = "$nowSec"
    $appid = "3116"
    $clientver = "11440"
    $userid = "0"
    $vip = "0"

    # signKey: hash + salt1856 + appid + mid + userid
    $keySalt = "185672dd44712f60bb1736df5a377e82"
    $keyInput = "$TEST_HASH$keySalt$appid$mid$userid"
    $keyHash = -join ([System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($keyInput)) | ForEach-Object { $_.ToString("x2") })

    # signature 签名
    $sigSalt = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA"
    $sortedPairs = @(
        "area_code=1",
        "behavior=play",
        "clienttime=$clienttime",
        "dfid=$dfid",
        "mid=$mid",
        "token=",
        "userid=$userid",
        "vip=$vip"
    ) | Sort-Object
    $sigInput = "$sigSalt$($sortedPairs -join '')$sigSalt"
    $sigHash = -join ([System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($sigInput)) | ForEach-Object { $_.ToString("x2") })

    Write-Host "  dfid=$dfid" -ForegroundColor Gray
    Write-Host "  mid=$($mid.Substring(0,8))... (len=$($mid.Length))" -ForegroundColor Gray
    Write-Host "  key=$($keyHash.Substring(0,8))... (len=$($keyHash.Length))" -ForegroundColor Gray
    Write-Host "  sig=$($sigHash.Substring(0,8))... (len=$($sigHash.Length))" -ForegroundColor Gray

    $body = @{
        area_code = "1"
        behavior = "play"
        qualities = @("128", "320", "flac")
        resource = @{
            album_audio_id = $TEST_ALBUM_ID
            collect_list_id = "3"
            collect_time = [int64](Get-Date).ToUniversalTime().Subtract((Get-Date "1/1/1970")).TotalMilliseconds
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
        userid = "$userid"
        vip = [int]$vip
    } | ConvertTo-Json -Depth 10

    try {
        $resp2 = Invoke-WebRequest -Uri $v6Url -Method POST `
            -Headers @{
                "User-Agent" = "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36"
                "Referer" = "https://www.kugou.com/"
                "dfid" = $dfid
                "clienttime" = $clienttime
                "mid" = $mid
            } `
            -Body $body `
            -ContentType "application/json" `
            -TimeoutSec 10

        $data = $resp2.Content | ConvertFrom-Json
        Write-Host "`n  响应: errcode=$($data.errcode)" -ForegroundColor Gray
        Write-Host "  响应内容: $($resp2.Content.Substring(0, [Math]::Min(200, $resp2.Content.Length)))" -ForegroundColor Gray

        if ($data.errcode -eq 0 -and $data.data.url) {
            Write-Host "✅ v6播放URL获取成功! URL长度=$($data.data.url.Length)" -ForegroundColor Green
        } else {
            $msg = if ($data.error) { $data.error } elseif ($data.msg) { $data.msg } else { $data | ConvertTo-Json }
            Write-Host "⚠️  v6失败 errcode=$($data.errcode): $msg" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ v6请求异常: $_" -ForegroundColor Red
        Write-Host "  状态: $($_.Exception.Response.StatusCode)" -ForegroundColor Gray
    }
}

# 测试3: legacy fallback
if ($TEST_HASH) {
    Write-Host "`n=== 测试3: legacy播放URL（无token fallback） ===" -ForegroundColor Cyan
    $legacyUrl = "https://www.kugou.com/yy/index.php?r=play/getdata&hash=$TEST_HASH&album_id=$TEST_ALBUM_ID"
    try {
        $legacyResp = Invoke-WebRequest -Uri $legacyUrl -Method GET `
            -Headers @{
                "User-Agent" = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                "Referer" = "https://www.kugou.com/"
            } `
            -TimeoutSec 10
        $legacyData = $legacyResp.Content | ConvertFrom-Json
        if ($legacyData.status -eq 1 -and $legacyData.data.play_url) {
            Write-Host "✅ legacy播放URL成功! URL长度=$($legacyData.data.play_url.Length)" -ForegroundColor Green
        } else {
            $err = $legacyData.error -or "status=$($legacyData.status)"
            Write-Host "⚠️  legacy失败: $err" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ legacy请求异常: $_" -ForegroundColor Red
    }
}

Write-Host "`n=== 测试完成 ===" -ForegroundColor Cyan
