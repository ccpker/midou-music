"""
酷狗 API 最终验证
目标: 确定酷狗播放 API 的真实可用性
"""
import urllib.request
import urllib.parse
import json
import time
import hashlib

def md5_hex(s):
    return hashlib.md5(s.encode()).hexdigest()

def sign_android(params_dict, body=""):
    SALT = "OIlwieks28dk2k092lksi2UIkp"
    sorted_keys = sorted(params_dict.keys())
    pairs = "".join(f"{k}={params_dict[k]}" for k in sorted_keys)
    return md5_hex(f"{SALT}{pairs}{body}{SALT}")

SALT_WEB = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt"

def sign_web(params_dict):
    sorted_keys = sorted(params_dict.keys())
    pairs = "".join(f"{k}={params_dict[k]}" for k in sorted_keys)
    return md5_hex(f"{SALT_WEB}{pairs}{SALT_WEB}")

# ──── 步骤 1: 先搜索拿真实歌曲数据 ────
print("=== 步骤1: 搜索歌曲 ===")

def search_kugou(keyword):
    params = {"keyword": keyword, "page": "1", "pagesize": "3", "platform": "WebFilter"}
    url = f"https://songsearch.kugou.com/song_search_v2?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Referer": "https://www.kugou.com/",
    })
    with urllib.request.urlopen(req, timeout=10) as resp:
        data = json.loads(resp.read().decode())
        return data.get("lists", [])

lists = search_kugou("夜空中最亮的星")
for s in lists[:3]:
    print(f"  [{s.get('SongName')}] by [{s.get('SingerName')}]")
    print(f"    FileHash={s.get('FileHash')} AlbumId={s.get('AlbumId')}")
    print(f"    Duration={s.get('Duration')}")

# ──── 步骤 2: 用真实数据测试 v5/url ────
print("\n=== 步骤2: 测试 v5/url with 真实数据 ===")

def test_v5(hash_val, album_id, song_name):
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
        "User-Agent": "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
        "dfid": dfid,
        "clienttime": ct,
        "mid": mid,
        "kg-rc": "1",
        "kg-thash": "5d816a0",
        "kg-rec": "1",
        "Referer": "https://www.kugou.com/",
        "x-router": "trackercdn.kugou.com",
    })
    
    with urllib.request.urlopen(req, timeout=10) as resp:
        data = json.loads(resp.read().decode())
        play_url = data.get("url", "")
        errcode = data.get("errcode", "?")
        print(f"  [{song_name}] errcode={errcode}")
        if play_url:
            print(f"    ✅ URL: {play_url[:100]}")
        else:
            print(f"    ❌ 无URL")
            print(f"    响应: {json.dumps(data, ensure_ascii=False)[:200]}")

for s in lists[:2]:
    test_v5(s.get("FileHash",""), str(s.get("AlbumId","")), s.get("SongName",""))

# ──── 步骤 3: 测试 v1 旧 API ────
print("\n=== 步骤3: 测试旧版 getdata API (v1用这个) ===")

def test_v1(hash_val, album_id, song_name):
    ct = str(int(time.time()))
    dfid = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"
    mid = md5_hex(dfid)
    uuid = md5_hex(f"{dfid}{mid}")
    
    params = {
        "r": "play/getdata",
        "hash": hash_val.lower(),
        "album_id": album_id,
        "dfid": dfid,
        "mid": mid,
        "clientver": "11309",
        "appid": "1005",
        "clienttime": ct,
    }
    
    sig = sign_web(params)
    params["signature"] = sig
    
    url = "https://www.kugou.com/yy/index.php?" + urllib.parse.urlencode(params)
    
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Referer": "https://www.kugou.com/",
        "Cookie": f"dfid={dfid}; mid={mid}",
    })
    
    with urllib.request.urlopen(req, timeout=10) as resp:
        raw = resp.read().decode()
        try:
            data = json.loads(raw)
        except:
            # 可能是非 JSON 响应
            print(f"  [{song_name}] 非JSON响应: {raw[:200]}")
            return
        
        # data 结构: {"err_code": int, "data": {...}}
        err = data.get("err_code", 0)
        play_data = data.get("data", {})
        play_url = play_data.get("play_url", "") if isinstance(play_data, dict) else ""
        
        print(f"  [{song_name}] err_code={err}")
        if play_url:
            print(f"    ✅ URL: {play_url[:100]}")
        else:
            print(f"    ❌ 无URL, data={str(play_data)[:300]}")
            print(f"    完整响应: {json.dumps(data, ensure_ascii=False)[:300]}")

for s in lists[:2]:
    test_v1(s.get("FileHash",""), str(s.get("AlbumId","")), s.get("SongName",""))

# ──── 步骤 4: 尝试 v3/get_song_info ────
print("\n=== 步骤4: 测试 v3/get_song_info (免登录) ===")

def test_v3(hash_val, song_name):
    ct = str(int(time.time()))
    dfid = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"
    mid = md5_hex(dfid)
    
    params = {
        "hash": hash_val.lower(),
        "dfid": dfid,
        "mid": mid,
        "clientver": "11430",
        "appid": "1005",
        "clienttime": ct,
        "cmd": "25",
        "behavior": "download",
    }
    
    sig = sign_android(params)
    params["signature"] = sig
    
    url = "https://trackercdn.kugou.com/v3/get_song_info?" + urllib.parse.urlencode(params)
    
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Linux; Android 13)",
        "dfid": dfid,
        "clienttime": ct,
        "mid": mid,
        "x-router": "trackercdn.kugou.com",
    })
    
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())
            play_url = data.get("url", "") or data.get("play_url", "")
            errcode = data.get("errcode", "?")
            print(f"  [{song_name}] errcode={errcode}")
            if play_url:
                print(f"    ✅ URL: {play_url[:100]}")
            else:
                print(f"    ❌ 无URL: {json.dumps(data, ensure_ascii=False)[:200]}")
    except Exception as e:
        print(f"  [{song_name}] Error: {e}")

for s in lists[:2]:
    test_v3(s.get("FileHash",""), s.get("SongName",""))
