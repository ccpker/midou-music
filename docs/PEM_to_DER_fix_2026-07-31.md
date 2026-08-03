# PEM→DER 修复记录 2026-07-31 10:06

## 问题

`ssa.rs` 和 `register_dev.rs` 中 RSA 公钥用 PEM 格式（`-----BEGIN PUBLIC KEY-----`），`rsa` crate 的 `from_public_key_pem()` 在某些环境下解析失败：
```
RSA raw PEM parse: ASN.1 error: PEM error: PEM Base64 error: invalid Base64 encoding
```

## 修复

两个文件全部改用 DER hex 编码：

| 文件 | 变更 |
|------|------|
| `ssa.rs` | `OAEP_PEM` → `OAEP_DER_HEX`，`LITE_PEM` → `LITE_DER_HEX` |
| `register_dev.rs` | `LITE_RSA_PEM` → `LITE_DER_HEX` |

新增 `parse_public_key(der_hex)` 辅助函数：
```rust
fn parse_public_key(der_hex: &str) -> Result<RsaPublicKey, String> {
    let der = hex::decode(der_hex).map_err(|e| format!("DER hex decode: {e}"))?;
    RsaPublicKey::from_public_key_der(&der).map_err(|e| format!("DER parse: {e}"))
}
```

## 编译

`cargo check` → 0 errors ✅

## SSA 验证触发

播放时弹出了「安全验证」窗口，说明 SSA 链路已触发（RiskVerifyModal 弹出）。用户需手动完成滑块验证后，验证通过会自动重试播放。
