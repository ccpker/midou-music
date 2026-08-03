"""
酷狗播放地址 - 使用 v1 的固定 dfid + 旧版 API
"""
import urllib.request
import urllib.parse
import json
import time
import hashlib

APPID = "1005"
CLIENTVER = "11309"   # v1 用的是 11309（旧版）
SRC_APPID = "2919"
SALT = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt"  # v1 的 Web 签名盐

DEFAULT_DFID = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6"  # v1 固定值

def md5_hex(s):
    return hashlib.md5(s.encode()).hexdigest()

def sign_web(params_dict):
    """v1 的 Web 签名方式"""
    sorted_keys = sorted(params_dict.keys())
    pairs = "".join(f"{k}={params_dict[k]}" for k in sorted_keys)
    return md5_hex(f"{SALT}{pairs}{SALT}")

def default_params():
    dfid = DEFAULT_DFID
    mid = md5_hex(dfid)
    uuid = md5_hex(f"{dfid}{mid}")
    clienttime = str(int(time.time()))
    return {
        "dfid": dfid,
        "mid": mid,
        "uuid": uuid,
        "appid": APPID,
        "clientver": CLIENTVER,
        "clienttime": clienttime,
    }

# ========== 方案 A: 旧版 play/getdata API (v1 用这个) ==========
print("--- 方案 A: www.kugou.com/yy/index.php?r=play/getdata ---")

def test_v1_api(hash_val, album_id, song_name):
    print(f"\n{song_name}:")
    params = default_params()
    params.update({
        "hash": hash_val.lower(),
        "album_id": album_id,
    })
    sig = sign_web(params)
    params["signature"] = sig
    
    query = urllib.parse.urlencode(params)
    url = f"https://www.kugou.com/yy/index.php?r=play/getdata&{query}"
    
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Referer": "https://www.kugou.com/",
        "Cookie": f"dfid={DEFAULT_DFID}",
    })
    
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            body = resp.read().decode()
            data = json.loads(body)
            play_url = data.get("data", {}).get("play_url", "")
            err = data.get("err_code", 0)
            print(f"  err_code={err}, url={'有!' if play_url else '无'}")
            if play_url:
                print(f"  -> {play_url[:100]}")
    except Exception as e:
        print(f"  ❌ {e}")

# 测试
test_v1_api("766877C1D3DD28BE7546506922EA17D7", "961718", "逃跑计划 - 夜空中最亮的星")
test_v1_api("B3A52A7A958BF0AED0EBFBA2E9A818B7", "966846", "周杰伦 - 晴天")

# ========== 方案 B: v5/url API（酷狗概念版，notSign=true）==========
print("\n\n--- 方案 B: trackercdn.kugou.com/v5/url (notSign) ---")

def test_v5_nosign(hash_val, album_id, song_name):
    print(f"\n{song_name}:")
    ct = str(int(time.time()))
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
        "dfid": DEFAULT_DFID,
        "mid": md5_hex(DEFAULT_DFID),
        "uuid": "-",
        "appid": "1005",
        "clienttime": ct,
    }
    
    query = urllib.parse.urlencode(params)
    url = f"https://trackercdn.kugou.com/v5/url?{query}"
    
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36",
        "dfid": DEFAULT_DFID,
        "clienttime": ct,
        "mid": md5_hex(DEFAULT_DFID),
        "kg-rc": "1",
        "kg-thash": "5d816a0",
        "x-router": "trackercdn.kugou.com",
    })
    
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            body = resp.read().decode()
            data = json.loads(body)
            play_url = data.get("url", "")
            errcode = data.get("errcode", "?")
            print(f"  errcode={errcode}, url={'有!' if play_url else '无'}")
            if play_url:
                print(f"  -> {play_url[:100]}")
    except Exception as e:
        print(f"  ❌ {e}")

test_v5_nosign("766877C1D3DD28BE7546506922EA17D7", "961718", "逃跑计划 - 夜空中最亮的星")
test_v5_nosign("B3A52A7A958BF0AED0EBFBA2E9A818B7", "966846", "周杰伦 - 晴天")
