// SSA 完整流程 — 纯 Node crypto（无外部依赖）
const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }
function randomString(len) { return crypto.randomBytes(Math.ceil(len/2)).toString('hex').slice(0, len); }

// --- sigAndroid ---
function sigAndroid(params, body) {
    const salt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA'; // lite
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => `${k}=${typeof params[k] === 'string' ? params[k] : JSON.stringify(params[k])}`).join('');
    return md5(salt + kv + (body || '') + salt);
}

// --- HTTP ---
function httpReq(method, hostname, path, headers, body) {
    return new Promise((resolve, reject) => {
        const req = https.request({ hostname, path, method, headers }, res => {
            let d = ''; res.on('data', c => d += c);
            res.on('end', () => { try { resolve(JSON.parse(d)); } catch { resolve(d); } });
        });
        req.on('error', reject);
        if (body) req.write(body);
        req.end();
    });
}

// --- RSA raw encrypt (m^e mod n) using crypto.createPublicKey for PEM parse ---
function rsaRawEncrypt(data, pem) {
    // Parse PEM with Node crypto
    const keyObj = crypto.createPublicKey({ key: pem, format: 'pem', type: 'pkcs1' });
    // Export as JWK to get n and e
    const jwk = keyObj.export({ format: 'jwk' });
    return { jwk }; // just debug first
}

// First just decode the key
const pubLite = `-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDECi0Np2UR87scwrvTr72L6oO0
1rBbbBPriSDFPxr3Z5syug0O24QyQO8bg27+0+4kBzTBTBOZ/WWU0WryL1JSXRTX
LgFVxtzIY41Pe7lPOgsfTCn5kZcvKhYKJesKnnJDNr5/abvTGf+rHG3YRwsCHcQ0
8/q6ifSioBszvb3QiwIDAQAB
-----END PUBLIC KEY-----`;

const keyObj = crypto.createPublicKey(pubLite);
console.log('key type:', keyObj.type);
console.log('asymmetricKeyType:', keyObj.asymmetricKeyType);

// Export as JWK → get n (base64url)
const jwk = keyObj.export({ format: 'jwk' });
console.log('JWK n[..40]:', jwk.n ? jwk.n.slice(0, 40) : 'N/A');
console.log('JWK e:', jwk.e);

// Decode base64url n → BigInt
function b64urlDecode(str) {
    str = str.replace(/-/g, '+').replace(/_/g, '/');
    return Buffer.from(str, 'base64');
}
const nBuf = b64urlDecode(jwk.n);
const eBuf = b64urlDecode(jwk.e);
const n_val = BigInt('0x' + nBuf.toString('hex'));
const e_val = BigInt('0x' + eBuf.toString('hex'));
console.log('n bits:', n_val.toString(2).length);
console.log('e:', e_val.toString());

// Test RSA encrypt: data = JSON.stringify({key:"test"}) → hex → BigInt → modPow
const testData = JSON.stringify({ key: 'testkey1234567' });
const testBuf = Buffer.from(testData, 'utf8');
const keyLen = 128; // 1024 bits
const padded = Buffer.alloc(keyLen);
testBuf.copy(padded);
const m = BigInt('0x' + padded.toString('hex'));

// modPow
function modPow(base, exp, mod) {
    if (mod === 1n) return 0n;
    let r = 1n;
    base = base % mod;
    while (exp > 0n) { if (exp & 1n) r = (r * base) % mod; exp >>= 1n; base = (base * base) % mod; }
    return r;
}
const encrypted = modPow(m, e_val, n_val);
const pk = encrypted.toString(16).padStart(keyLen * 2, '0');
console.log('RSA pk hex[..40]:', pk.slice(0, 40));
console.log('RSA works!');
