"""
酷狗 API 最小化测试 - 不带任何 Cookie
"""
import urllib.request
import urllib.parse
import json
import time
import hashlib

def md5_hex(s):
    return hashlib.md5(s.encode()).hexdigest()

SALT_WEB = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt"
SALT_ANDROID = "OIlwieks28dk2k092lksi2UIkp"

def sign_web(params_dict):
    sorted_keys = sorted(params_dict.keys())
    pairs = "".join(f"{k}={params_dict[k]}" for k in sorted_keys)
    return md5_hex(f"{SALT_WEB}{pairs}{SALT_WEB}")

def sign_android(params_dict, body=""):
    sorted_keys = sorted(params_dict.keys())
    pairs = "".join(f"{k}={params_dict[k]}" for k in sorted_keys)
    return md5_hex(f"{SALT_ANDROID}{pairs}{body}{SALT_ANDROID}")

# ========== 测试 1: 简单搜索 ==========
print("--- 测试 1: 简单搜索 (songsearch.kugou.com) ---")

def test_search(keyword):
    params = {
        "keyword": keyword,
        "page": "1",
        "pagesize": "5",
        "platform": "WebFilter",
    }
    query = urllib.parse.urlencode(params)
    url = f"https://songsearch.kugou.com/song_search_v2?{query}"
    
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Referer": "https://www.kugou.com/",
    })
    
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            body = resp.read().decode()
            data = json.loads(body)
            songs = data.get("lists", [])
            print(f"  '{keyword}': {len(songs)} 首")
            for s in songs[:3]:
                print(f"    - {s.get('SongName')} | hash={s.get('FileHash','')[:16]}... album={s.get('AlbumId','')}")
            return songs
    except Exception as e:
        print(f"  ❌ {e}")
        return []

lists = test_search("周杰伦")

# ========== 测试 2: v5/url with minimal params ==========
print("\n--- 测试 2: v5/url (最小参数) ---")

def test_v5_min(hash_val, album_id, song_name):
    print(f"\n  {song_name}")
    ct = str(int(time.time()))
    
    # 尝试最简单的参数组合
    params = {
        "hash": hash_val.lower(),
        "album_id": album_id,
        "quality": "128",
        "cmd": "26",
        "behavior": "play",
        "clientver": "11430",
        "dfid": "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6",
        "mid": md5_hex("2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"),
        "appid": "1005",
        "clienttime": ct,
    }
    
    url = f"https://trackercdn.kugou.com/v5/url?" + urllib.parse.urlencode(params)
    
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Linux; Android 13)",
        "dfid": params["dfid"],
        "clienttime": ct,
        "mid": params["mid"],
        "x-router": "trackercdn.kugou.com",
    })
    
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            body = resp.read().decode()
            data = json.loads(body)
            play_url = data.get("url", "")
            errcode = data.get("errcode", "?")
            print(f"    errcode={errcode}, url={'有!' if play_url else '无'}")
            if play_url:
                print(f"    -> {play_url[:100]}")
    except Exception as e:
        print(f"    ❌ {e}")

if lists:
    s = lists[0]
    test_v5_min(s.get("FileHash",""), str(s.get("AlbumId","")), s.get("SongName",""))

# ========== 测试 3: 带签名的 v5/url ==========
print("\n\n--- 测试 3: v5/url with android signature ---")

def test_v5_signed(hash_val, album_id, song_name):
    print(f"\n  {song_name}")
    ct = str(int(time.time()))
    dfid = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"
    mid = md5_hex(dfid)
    
    params = {
        "album_id": album_id,
        "area_code": "1",
        "hash": hash_val.lower(),
        "ssa_flag": "is_fromtrack",
        "version": "11430",
        "page_id": "151369488",
        "quality": "128",
        "album_audio_id": "0",
        "behavior": "play",
        "pid": "2",
        "cmd": "26",
        "pidversion": "3001",
        "IsFreePart": "0",
        "ppage_id": "463467626,350369493,788954147",
        "cdnBackup": "1",
        "module": "",
        "clientver": "11430",
        "dfid": dfid,
        "mid": mid,
        "uuid": "-",
        "appid": "1005",
        "clienttime": ct,
    }
    
    sig = sign_android(params)
    params["signature"] = sig
    
    url = "https://trackercdn.kugou.com/v5/url?" + urllib.parse.urlencode(params)
    
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Linux; Android 13)",
        "dfid": dfid,
        "clienttime": ct,
        "mid": mid,
        "kg-rc": "1",
        "kg-thash": "5d816a0",
        "x-router": "trackercdn.kugou.com",
    })
    
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            body = resp.read().decode()
            data = json.loads(body)
            play_url = data.get("url", "")
            errcode = data.get("errcode", "?")
            print(f"    errcode={errcode}, url={'有!' if play_url else '无'}")
            if play_url:
                print(f"    -> {play_url[:100]}")
    except Exception as e:
        print(f"    ❌ {e}")

if lists:
    s = lists[0]
    test_v5_signed(s.get("FileHash",""), str(s.get("AlbumId","")), s.get("SongName",""))
