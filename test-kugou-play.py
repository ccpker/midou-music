"""
酷狗音乐 API 播放地址测试
参考: MakcRe/KuGouMusicApi (https://github.com/MakcRe/KuGouMusicApi)
"""

import urllib.request
import urllib.parse
import json
import time
import random
import string
import hashlib

# ========== 酷狗配置 ==========
APPID = 1005
CLIENTVER = 20489   # 标准版（非 lite）
SALT = "OIlwieks28dk2k092lksi2UIkp"  # 标准版签名盐值
UA = "Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36"

# ========== 工具函数 ==========

def random_string(n=24):
    chars = '1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ'
    return ''.join(random.choice(chars) for _ in range(n))

def md5_hex(s):
    return hashlib.md5(s.encode()).hexdigest()

def calculate_mid(guid):
    """MD5(guid) -> hex -> u128 -> decimal"""
    digest = md5_hex(guid)
    result = 0
    for c in digest:
        result = result * 16 + int(c, 16)
    return str(result)

def android_signature(params, body=""):
    """Android 签名: MD5(salt + sorted(k=v) + body + salt)"""
    sorted_params = sorted(params.items())
    param_str = "".join(f"{k}={v}" for k, v in sorted_params)
    raw = f"{SALT}{param_str}{body}{SALT}"
    return md5_hex(raw)

# ========== 生成设备标识 ==========
guid = f"{random_string(8)}-{random_string(4)}-4{random_string(3)}-{random_string(4)}-{random_string(12)}"
dfid = random_string(24)
mid = calculate_mid(guid)
clienttime = str(int(time.time()))

print(f"设备: dfid={dfid[:12]}..., mid={mid[:20]}..., guid={guid[:20]}...")

# ========== 测试歌曲 ==========
# 周杰伦 - 晴天
song_hash = "B3A52A7A958BF0AED0EBFBA2E9A818B7"
song_album = "966846"

# 逃跑计划 - 夜空中最亮的星
song2_hash = "766877C1D3DD28BE7546506922EA17D7"
song2_album = "961718"

def test_v5_url(hash_val, album_id, song_name):
    print(f"\n{'='*50}")
    print(f"测试: {song_name}")
    print(f"hash={hash_val}, album={album_id}")
    
    # 构建参数
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
        "clientver": str(CLIENTVER),
        "dfid": dfid,
        "mid": mid,
        "uuid": "-",
        "appid": str(APPID),
        "clienttime": clienttime,
    }
    
    # 生成签名
    sig = android_signature(params)
    params["signature"] = sig
    
    query = urllib.parse.urlencode(params)
    url = f"https://trackercdn.kugou.com/v5/url?{query}"
    
    print(f"URL: {url[:120]}...")
    
    req = urllib.request.Request(url, headers={
        "User-Agent": UA,
        "dfid": dfid,
        "clienttime": clienttime,
        "mid": mid,
        "kg-rc": "1",
        "kg-thash": "5d816a0",
        "kg-rec": "1",
        "Referer": "https://www.kugou.com/",
        "x-router": "trackercdn.kugou.com",
    })
    
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            body = resp.read().decode()
            data = json.loads(body)
            print(f"Status: {resp.status}, errcode: {data.get('errcode')}, errmsg: {data.get('errmsg')}")
            url2 = data.get("url", "")
            if url2:
                print(f"✅ 播放URL有效! 长度={len(url2)}, URL={url2[:100]}...")
            else:
                print(f"❌ url为空")
    except Exception as e:
        print(f"❌ 请求失败: {e}")

# 测试两首歌
test_v5_url(song_hash, song_album, "周杰伦 - 晴天")
test_v5_url(song2_hash, song2_album, "逃跑计划 - 夜空中最亮的星")
