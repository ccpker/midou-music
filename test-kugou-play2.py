"""
酷狗设备注册 + 播放地址测试
"""
import urllib.request
import urllib.parse
import json
import time
import random
import string
import hashlib

APPID = 1005
CLIENTVER = 20489
SALT = "OIlwieks28dk2k092lksi2UIkp"

def random_string(n=24):
    chars = '1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ'
    return ''.join(random.choice(chars) for _ in range(n))

def md5_hex(s):
    return hashlib.md5(s.encode()).hexdigest()

def calculate_mid(guid):
    digest = md5_hex(guid)
    result = 0
    for c in digest:
        result = result * 16 + int(c, 16)
    return str(result)

def android_sig(params, body=""):
    sorted_params = sorted(params.items())
    param_str = "".join(f"{k}={v}" for k, v in sorted_params)
    raw = f"{SALT}{param_str}{body}{SALT}"
    return md5_hex(raw)

guid = f"{random_string(8)}-{random_string(4)}-4{random_string(3)}-{random_string(4)}-{random_string(12)}"
dfid = random_string(24)
mid = calculate_mid(guid)
clienttime = str(int(time.time()))

print(f"设备: dfid={dfid}, mid={mid}")

# ========== Step 1: 注册设备 ==========
print("\n--- Step 1: register_dev ---")
reg_params = {
    "appid": str(APPID),
    "clientver": str(CLIENTVER),
    "clienttime": clienttime,
    "dfid": dfid,
    "mid": mid,
    "platid": "5",
    "signature": "",
}
sorted_vals = sorted(str(v) for v in reg_params.values())
vals_str = "".join(sorted_vals)
reg_sig = md5_hex(f"1014{vals_str}1014")
reg_params["signature"] = reg_sig

query = urllib.parse.urlencode(reg_params)
url = f"https://gateway.kugou.com/openmak/regeist/dev?{query}"

req = urllib.request.Request(url, headers={
    "User-Agent": "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36",
    "dfid": dfid,
    "clienttime": clienttime,
    "mid": mid,
    "Referer": "https://www.kugou.com/",
})

try:
    with urllib.request.urlopen(req, timeout=15) as resp:
        body = resp.read().decode()
        data = json.loads(body)
        print(f"Status: {resp.status}")
        print(f"Response: {json.dumps(data, indent=2, ensure_ascii=False)[:500]}")
        if data.get("status") == 1:
            print("✅ 注册成功!")
        set_cookie = resp.headers.get("Set-Cookie", "")
        print(f"Set-Cookie: {set_cookie[:300]}")
except Exception as e:
    print(f"Error: {e}")

# ========== Step 2: 用注册后的 dfid 测试 v5/url ==========
print("\n--- Step 2: v5/url ---")

def test_v5(hash_val, album_id, quality="128"):
    ct = str(int(time.time()))
    params = {
        "album_id": album_id,
        "area_code": "1",
        "hash": hash_val.lower(),
        "ssa_flag": "is_fromtrack",
        "version": "11430",
        "page_id": "151369488",
        "quality": quality,
        "album_audio_id": "0",
        "behavior": "play",
        "pid": "2",
        "cmd": "26",
        "pidversion": "3001",
        "IsFreePart": "1",
        "ppage_id": "463467626,350369493,788954147",
        "cdnBackup": "1",
        "module": "",
        "clientver": str(CLIENTVER),
        "dfid": dfid,
        "mid": mid,
        "uuid": "-",
        "appid": str(APPID),
        "clienttime": ct,
    }
    sig = android_sig(params)
    params["signature"] = sig

    query = urllib.parse.urlencode(params)
    url = f"https://trackercdn.kugou.com/v5/url?{query}"

    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36",
        "dfid": dfid,
        "clienttime": ct,
        "mid": mid,
        "kg-rc": "1",
        "kg-thash": "5d816a0",
        "x-router": "trackercdn.kugou.com",
    })

    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            body = resp.read().decode()
            data = json.loads(body)
            url2 = data.get("url", "")
            print(f"  quality={quality}: errcode={data.get('errcode')}, url={'有!' if url2 else '无'}")
            if url2:
                print(f"    -> {url2[:100]}")
    except Exception as e:
        print(f"  quality={quality}: Error {e}")

# 测试周杰伦 - 晴天
test_v5("B3A52A7A958BF0AED0EBFBA2E9A818B7", "966846", "128")
test_v5("B3A52A7A958BF0AED0EBFBA2E9A818B7", "966846", "320")
