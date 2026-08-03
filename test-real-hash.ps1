$headers = @{
    'User-Agent' = 'Mozilla/5.0'
    'Referer' = 'https://www.kugou.com/'
}
$uri = 'https://songsearch.kugou.com/song_search_v2?keyword=%E5%91%A8%E6%9D%B0%E4%BC%A6&platform=WebFilter&format=json&page=1&pagesize=1&userid=-1'
try {
    $r = Invoke-WebRequest -Uri $uri -Headers $headers -TimeoutSec 10 -UseBasicParsing
    $j = $r.Content | ConvertFrom-Json
    $song = $j.data.lists[0]
    Write-Host "hash=$($song.FileHash)"
    Write-Host "album=$($song.AlbumId)"
    Write-Host "name=$($song.FileName)"
} catch {
    Write-Host "Error: $($_.Exception.Message)"
}
