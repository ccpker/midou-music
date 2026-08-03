# P0-1: register_dev 实现 — 进展追踪

**开始**: 2026-07-31 10:00 | **状态**: ✅ 实现完成，编译通过，**待运行时验证**

## 已完成的实现

### 1. `src-tauri/src/register_dev.rs`（新文件）

完整复刻 KuGouMusicApi 的 `module/register_dev.js`：

- `playlist_aes_encrypt(data_json)` → (key, b64_body)  — 随机6字符key → MD5 → 前16char=key, 后16char=iv → AES-128-CBC PKCS7 → Base64
- `playlist_aes_decrypt(b64_data, key)` → JSON  — Base64 → AES解密 → UTF-8 → JSON
- `rsa_pkcs1_encrypt(data_json)` → hex  — 使用 Lite 1024-bit RSA公钥，PKCS1v1.5
- `calculate_mid_from_guid(guid)` → 十进制字符串  — GUID → MD5 hex → BigUint → 十进制（复刻 util/util.js::calculateMid）
- `generate_guid()` → UUID v4
- `register_device(client, token, userid, guid, old_dfid, mid)` → dfid

签名算法（复刻 helper.js signatureAndroidParams）：
```
MD5(LITE_SIGN_SALT + sorted(k=v) + body + LITE_SIGN_SALT)
```
盐值: `LnT6xpN3khm36zse0QzvmgTZ3waWdRSA` (lite)

### 2. Tauri 命令 `kugou_register_dev`

- 参数: guid, old_dfid
- 调用 `register_dev::register_device()`
- 成功后更新内存 auth.dfid 和 auth.kugou_api_mid
- 返回 { dfid, mid, guid } → 前端存 Store

### 3. `kugou_set_auth` 扩展

新增可选参数 `dfid` / `mid`，前端 register_dev 后回传给 Rust：
```rust
dfid: dfid.unwrap_or_else(|| DFID.into()),
kugou_api_mid: mid.unwrap_or_default(),
```

### 4. 前端初始化流程

App.tsx useEffect:
1. 读 Store `kugou_device` → 有 dfid/mid 则跳过注册
2. 没有 → `invoke("kugou_register_dev")` → 存 Store
3. 读 Store `kugou_auth` → 有 token → `invoke("kugou_set_auth")` 带 dfid/mid

KugouLogin.tsx:
- `setupAuth()` 函数 — 从 Store 读 `kugou_device`，推给 Rust

## 传感器参数（直抄 register_dev.js）

- 品牌: Redmi, 设备: marble, 厂商: Xiaomi
- availableRamSize: 4983533568, batteryLevel: 100
- 20+ 传感器均为 false（accelerometer/gravity/gyroscope/light/magnetic/orientation/pressure/step_counter/temperature）

## 编译状态

```
cargo check → 0 errors, 9 warnings (inactive dead_code)
npx tsc --noEmit → 0 errors
npm run build → ✅ 三关全绿
```

## 下一步（执行清单进度）

| 任务 | 状态 |
|------|------|
| P0 register_dev Rust 实现 | ✅ 完成 |
| P0 前端初始化流程 | ✅ 完成 |
| ⏳ 运行时验证（启动APP → register_dev → 歌单20017是否消失） | **待验证** |
| P1 SSA 全链路联调 | 待做 |
| P2 酷我零登录接入 | 待做 |
| P2 B站风控 | 待做 |
