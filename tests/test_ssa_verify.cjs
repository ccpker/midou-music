// SSA Step 3 完整版：补上 cryptoAesEncrypt + cryptoRSAEncrypt
const crypto = require('crypto');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }
function smd5be(bytes) { const h = crypto.createHash('md5'); h.update(bytes); return h.digest('hex').substring(0, 16); }
function randomString(len) { return crypto.randomBytes(Math.ceil(len/2)).toString('hex').slice(0, len); }

// --- AES-128-CBC (standard lib) ---
function aesEncryptCBC(data, key, iv) {
// AES key = MD5(tempKey).substring(0,32) = 32 bytes → AES-256
    const cipher = crypto.createCipheriv('aes-256-cbc', Buffer.from(key, 'utf8'), Buffer.from(iv, 'utf8'));
    return Buffer.concat([cipher.update(data, 'utf8'), cipher.final()]).toString('hex');
}

// --- RSA (raw/standard, 用 node-forge 的 crypto.js 做 RSA-raw) ---
// MoeKoeMusic 的 cryptoRSAEncrypt 用 forge.js 的 rsa.encrypt 做 raw 加密（非 PKCS/OAEP）
// 我们需要模拟：RSA raw encrypt (big integer mod pow)
function rsaRawEncrypt(hexData, pem) {
    // 解析 PEM
    const b64 = pem.replace(/-----[^-]+-----/g, '').replace(/\s/g, '');
    const der = Buffer.from(b64, 'base64');
    // 简单 ASN.1 解析 SPKI：SEQUENCE → SEQUENCE OID NULL → BITSTRING
    let pos = 0;
    if (der[pos] !== 0x30) throw 'not SEQ';
    pos++; let len = der[pos++]; if (len > 0x80) { let llen = len - 0x80; len = der.readUIntBE(pos, llen); pos += llen; }
    pos++; let len2 = der[pos++]; if (len2 > 0x80) { let llen = len2 - 0x80; len2 = der.readUIntBE(pos, llen); pos += llen; }
    // skip OID + NULL (13 bytes for rsaEncryption)
    pos += 13;
    // BITSTRING: tag 03, len, unused_bits(00)
    if (der[pos] !== 0x03) throw 'not BITSTRING';
    pos++; let blen = der[pos++]; if (blen > 0x80) { let llen = blen - 0x80; blen = der.readUIntBE(pos, llen); pos += llen; }
    pos++; // unused bits = 0
    // Now SEQUENCE { INTEGER n, INTEGER e }
    if (der[pos] !== 0x30) throw 'not inner SEQ';
    pos++; let ilen = der[pos++]; if (ilen > 0x80) { let llen = ilen - 0x80; ilen = der.readUIntBE(pos, llen); pos += llen; }
    // INTEGER n
    if (der[pos] !== 0x02) throw 'not INT n';
    pos++; let nlen = der[pos++];
    const n = der.slice(pos, pos + nlen); pos += nlen;
    // INTEGER e
    if (der[pos] !== 0x02) throw 'not INT e';
    pos++; let elen = der[pos++];
    const e = der.slice(pos, pos + elen);

    // m^e mod n
    const m = BigInt('0x' + hexData);
    const nVal = BigInt('0x' + n.toString('hex'));
    const eVal = BigInt('0x' + e.toString('hex'));
    const result = modPow(m, eVal, nVal);
    const hex = result.toString(16).padStart(n.length * 2, '0');
    return hex;
}

function modPow(base, exp, mod) {
    if (mod === 1n) return 0n;
    let result = 1n;
    base = base % mod;
    while (exp > 0n) {
        if (exp & 1n) result = (result * base) % mod;
        exp >>= 1n;
        base = (base * base) % mod;
    }
    return result;
}

// --- 主流程 ---
const APPID = '3116', DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = '4b0a5cb94103098b612eecd6f9d4cc08', TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990';
const SSA_CODE = 'bj_tx_event_adb92cbd5c3dc2cb75bb38f717676529';

// --- cryptoAesEncrypt({}) ---
// MoeKoeMusic: tempKey = randomString(16).toLowerCase(), key = MD5(tempKey).substring(0,32), iv = key.substring(key.length-16)
// AES CBC, Pkcs7
const tempKey = randomString(16).toLowerCase();
const aesKey_md5 = md5(tempKey);
const aesKey = aesKey_md5.substring(0, 32);
const aesIv = aesKey.substring(aesKey.length - 16);

console.log('tempKey:', tempKey);
console.log('aesKey:', aesKey);
console.log('aesIv:', aesIv);

// encrypt empty object as JSON string "{}"
const plaintext = '{}';
const params_hex = aesEncryptCBC(plaintext, aesKey, aesIv);
console.log('params (AES encrypted {}):', params_hex.slice(0, 60));

// --- cryptoRSAEncrypt({ key: tempKey }) ---
// MoeKoeMusic: const buffer = normalizeBuffer(data) → UTF8 of JSON string
// uses lite public key
const pubLite = '-----BEGIN PUBLIC KEY-----\nMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDECi0Np2UR87scwrvTr72L6oO01rBbbBPriSDFPxr3Z5syug0O24QyQO8bg27+0+4kBzTBTBOZ/WWU0WryL1JSXRTXLgFVxtzIY41Pe7lPOgsfTCn5kZcvKhYKJesKnnJDNr5/abvTGf+rHG3YRwsCHcQ08/q6ifSioBszvb3QiwIDAQAB\n-----END PUBLIC KEY-----';

const rsaData = JSON.stringify({ key: tempKey });
const rsaHex = Buffer.from(rsaData, 'utf8').toString('hex');
console.log('RSA input hex:', rsaHex);

const pk = rsaRawEncrypt(rsaHex, pubLite);
console.log('pk:', pk.slice(0, 60));

// --- Build verify_user_info request ---
function sigAndroid(params, body) {
    const salt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA'; // lite
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => `${k}=${typeof params[k] === 'object' ? JSON.stringify(params[k]) : params[k]}`).join('');
    return md5(salt + kv + (body || '') + salt);
}

const https = require('https');
function httpReq(method, hostname, path, headers, body) {
    return new Promise((resolve, reject) => {
        const req = https.request({ hostname, path, method, headers }, res => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => {
                try { resolve(JSON.parse(data)); }
                catch { resolve(data); }
            });
        });
        req.on('error', reject);
        if (body) req.write(body);
        req.end();
    });
}

async function main() {
    const dataMap = JSON.stringify({
        eventid: SSA_CODE,
        userid: Number(USERID),
        platid: 2,
        v_type: 23,
        wasm: 1,
        i: '',
        sid: '',
        edt: '',
        verifycode: '',
        pk: pk,
        params: params_hex,
    });

    const t = Math.floor(Date.now() / 1000);
    const p = { appid: APPID, clientver: '11510', clienttime: t, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    const qs = Object.keys(p).sort().map(k => `${k}=${p[k]}`).join('&') + '&signature=' + sigAndroid(p, dataMap);

    console.log('\n=== verify_user_info ===');
    const r = await httpReq('POST', 'verifyservice.kugou.com', '/v4/verify_user_info?' + qs, {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'Content-Type': 'application/json',
        dfid: DFID, mid: MID, clienttime: String(t),
        'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
    }, dataMap);

    console.log(JSON.stringify(r, null, 2));
}

main().catch(e => console.error(e));
