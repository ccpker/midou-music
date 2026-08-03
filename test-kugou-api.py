import urllib.request
import urllib.parse
import json

keyword = "周杰伦"
encoded = urllib.parse.quote(keyword)

# 方法1: mobilecdn API
url1 = f"http://mobilecdn.kugou.com/api/v3/search/song?format=json&keyword={encoded}&page=1&pagesize=5"
print("=== 测试 mobilecdn API ===")
req1 = urllib.request.Request(url1, headers={
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
})
try:
    with urllib.request.urlopen(req1, timeout=10) as resp:
        body = resp.read().decode()
        print("Status:", resp.status)
        data = json.loads(body)
        print("error_code:", data.get("error_code"))
        info = data.get("info", [])
        print("info count:", len(info) if info else 0)
        if info:
            for i in info[:3]:
                print(f"  - {i.get('songname')} ({i.get('duration')}秒)")
except Exception as e:
    print("Error:", e)

# 方法2: songsearch API
print("\n=== 测试 songsearch API ===")
url2 = f"http://songsearch.kugou.com/song_search_v2?keyword={encoded}&platform=WebFilter&format=json&page=1&pagesize=5"
req2 = urllib.request.Request(url2, headers={
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
    "Referer": "https://www.kugou.com/",
})
try:
    with urllib.request.urlopen(req2, timeout=10) as resp:
        body = resp.read().decode()
        print("Status:", resp.status)
        data = json.loads(body)
        print("error_code:", data.get("error_code"))
        songs = data.get("data", {}).get("lists", [])
        print("找到", len(songs), "首")
        for s in songs[:3]:
            print(f"  - {s.get('FileName')} ({s.get('Duration')}秒)")
except Exception as e:
    print("Error:", e)
