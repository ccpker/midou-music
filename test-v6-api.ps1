# test-v6-api.ps1 — 测试酷狗 v6 播放 API
$hash = "B1A7EE6E7F2F7F2E8C8D9F3A5B7C6D4E"
$charset = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ"
$seed = [UInt64]([DateTimeOffset]::Now.ToUnixTimeNanoseconds())
$dfid = ""
for($i = 0; $i -lt 24; $i++) {
    $seed = ($seed * [UInt64]1103515245 + [UInt64]12345)
    $idx = ([Int64]($seed -shr 16) % 36)
    if ($idx -lt 0) { $idx += 36 }
    $dfid = $dfid + $charset[$idx]
}

# md5_hex(dfid)
$md5 = [System.Security.Cryptography.MD5]::Create()
$hbytes = $md5.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($dfid))
$hex = -join ($hbytes | ForEach-Object { $_.ToString('x2') })

# mid = parseInt(hex, 16).toString()
$mid = [BigInt]::Parse("0x$hex").ToString()

Write-Host "dfid=$dfid"
Write-Host "hex=$hex"
Write-Host "mid=$mid"

# tracker_param.key
$tkInput = $hash + "185672dd44712f60bb1736df5a377e82" + "3116" + $mid + "0"
$tkHash = [System.Security.Cryptography.MD5]::Create()
$tkBytes = $tkHash.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($tkInput))
$trackerKey = -join ($tkBytes | ForEach-Object { $_.ToString('x2') })

Write-Host "tracker_key=$trackerKey"

# v6 body
$body = @{
    area_code = "1"
    behavior = "play"
    qualities = @("128","320","flac","high","multitrack")
    resource = @{
        album_audio_id = 0
        collect_list_id = "3"
        collect_time = 0
        hash = $hash
        id = 0
        page_id = 1
        type = "audio"
    }
    token = ""
    tracker_param = @{
        all_m = 1
        auth = ""
        is_free_part = 0
        key = $trackerKey
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

$bodyJson = $body | ConvertTo-Json -Compress
Write-Host "body=$bodyJson"

# signatureAndroidParams
$sorted = "appid3116clientver11440dfid$dfid" + "mid$mid"
$sigInput = "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA" + $sorted + $bodyJson + "LnT6xpN3khm36zse0QzvmgTZ3waWdRSA"
$sigHash = [System.Security.Cryptography.MD5]::Create()
$sigBytes = $sigHash.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($sigInput))
$sig = -join ($sigBytes | ForEach-Object { $_.ToString('x2') })

Write-Host "sig=$sig"

$clienttime = [Math]::Floor(([DateTimeOffset]::Now.ToUnixTimeSeconds()))

$headers = @{
    "User-Agent" = "Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi"
    "dfid" = $dfid
    "mid" = $mid
    "clienttime" = $clienttime.ToString()
    "kg-rc" = "1"
    "kg-thash" = "5d816a0"
    "kg-rec" = "1"
    "kg-rf" = "B9EDA08A64250DEFFBCADDEE00F8F25F"
    "x-router" = "tracker.kugou.com"
    "Content-Type" = "application/json"
}

try {
    $resp = Invoke-WebRequest -Uri "http://tracker.kugou.com/v6/priv_url" -Method Post -Headers $headers -Body $bodyJson -TimeoutSec 15 -UseBasicParsing
    Write-Host "Status=$($resp.StatusCode)"
    Write-Host "Response=$($resp.Content)"
} catch {
    Write-Host "Error: $($_.Exception.Message)"
}
