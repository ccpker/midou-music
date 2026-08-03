// 完整 SSA 链：复用 test_v5_moekoe 的请求 → 捕获 ssa-code → 走完 verify
const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }
function smd5be(bytes) { const h = crypto.createHash('md5'); h.update(bytes); return h.digest('hex').substring(0, 16); }
function randomString(len) { return crypto.randomBytes(Math.ceil(len/2)).toString('hex').slice(0, len); }

// --- MoeKoeMusic 签名（lite）---
const SALT = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
const ENC_SALT = '185672dd44712f60bb1736df5a377e82';

function signKey(hash, mid, uid, appid) {
    return md5(`${hash}${ENC_SALT}${appid}${mid}${uid || 0}`);
}

function sigAndroid(params, body) {
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => `${k}=${typeof params[k] === 'object' ? JSON.stringify(params[k]) : params[k]}`).join('');
    return md5(`${SALT}${kv}${body || ''}${SALT}`);
}

// --- HTTP ---
function httpFull(method, hostname, path, headers, body) {
    return new Promise((resolve, reject) => {
        const req = https.request({ hostname, path, method, headers, rejectUnauthorized: false }, res => {
            let d = ''; res.on('data', c => d += c);
            res.on('end', () => {
                resolve({ status: res.statusCode, headers: Object.fromEntries(Object.entries(res.headers)), body: d, json: (() => { try { return JSON.parse(d); } catch { return null; } })() });
            });
        });
        req.on('error', reject);
        if (body) req.write(body);
        req.end();
    });
}

// --- RSA ---
function rsaRaw(data, pem) {
    const jwk = crypto.createPublicKey({ key: pem, format: 'pem', type: 'pkcs1' }).export({ format: 'jwk' });
    const nVal = BigInt('0x' + Buffer.from(jwk.n.replace(/-/g,'+').replace(/_/g,'/'), 'base64').toString('hex'));
    const eVal = BigInt('0x' + Buffer.from(jwk.e.replace(/-/g,'+').replace(/_/g,'/'), 'base64').toString('hex'));
    const keyLen = 128;
    const buf = Buffer.alloc(keyLen);
    Buffer.from(typeof data === 'string' ? data : data.toString(), 'utf8').copy(buf);
    let r = 1n, b = BigInt('0x' + buf.toString('hex')) % nVal, e = eVal;
    while (e > 0n) { if (e & 1n) r = (r * b) % nVal; e >>= 1n; b = (b * b) % nVal; }
    return r.toString(16).padStart(keyLen * 2, '0');
}

function aesEncrypt(data, key, iv) {
    const c = crypto.createCipheriv('aes-256-cbc', Buffer.from(key, 'utf8'), Buffer.from(iv, 'utf8'));
    return Buffer.concat([c.update(data, 'utf8'), c.final()]).toString('hex');
}

// --- generateSimulate ---
function generateSimulate(mid, uid, dfid, webglHash) {
    const SENTINEL = 0xffffffff - Math.floor(Math.random() * 20);
    const ri = (a,b) => Math.floor(Math.random()*(b-a+1))+a;

    let entries = [], ts = 0, ei = 0;
    entries.push('5,0,0', `5,${SENTINEL},0`, '5,0,0', `5,${SENTINEL},0`);
    ts += ri(5,20);
    entries.push(`6,${ts},${ei},750,500`, `6,${SENTINEL},${ei},750,500`);
    ei++;
    for (let i = 0; i < 3; i++) { ts += ri(80,600); entries.push(`5,${ts},${ei}`, `5,${SENTINEL},${ei}`); ei++; }

    const points = ri(30,60);
    let sx = ri(200,600), sy = ri(200,500), ex = ri(500,700), ey = ri(80,150);
    const c1x = sx + (ex-sx)*0.3 + ri(-80,80), c1y = sy + (ey-sy)*0.2 + ri(-60,60);
    const c2x = sx + (ex-sx)*0.7 + ri(-60,60), c2y = sy + (ey-sy)*0.8 + ri(-40,40);

    let si = 0;
    for (let i = 0; i <= points; i++) {
        const t = i/points, u = 1-t;
        let x = u*u*u*sx + 3*u*u*t*c1x + 3*u*t*t*c2x + t*t*t*ex;
        let y = u*u*u*sy + 3*u*u*t*c1y + 3*u*t*t*c2y + t*t*t*ey;
        x += (Math.random()-0.5) * Math.max(0.5, 3-t*2.5);
        y += (Math.random()-0.5) * Math.max(0.5, 3-t*2.5);
        ts += ri(8,50);
        entries.push(`3,${ts},${si},${Math.round(x)},${Math.round(y)}`, `3,${SENTINEL},${si},${Math.round(x)},${Math.round(y)}`);
        if (i > 0 && i % 12 === 0) { ts += ri(20,60); entries.push(`5,${ts},${ei}`, `5,${SENTINEL},${ei}`); ei++; }
        si = (si+1) % 2;
    }
    ts += ri(5,30);
    entries.push(`3,${ts},1,${Math.round(ex+ri(-5,5))},${Math.round(ey+ri(-5,5))}`);
    entries.push(`3,${SENTINEL},1,${Math.round(ex)},${Math.round(ey)}`);

    const data = entries.join(':');
    const plain = `mid=${mid||0};userid=${uid||0};dfid=${dfid||0};webgl=${webglHash};webdriver=0;ts=${Date.now()};data=${data}`;
    const key = smd5be(Buffer.from(randomString(16), 'utf8'));
    const edtCipher = crypto.createCipheriv('aes-128-cbc', Buffer.from(key,'utf8'), Buffer.from('kugousecurity123','utf8'));
    const edt = Buffer.concat([edtCipher.update(plain, 'utf8'), edtCipher.final()]).toString('base64');
    const sid = crypto.publicEncrypt({ key: '-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoW2+Ylo8ALePSQTP0xBF\nlFmEOHvBD9tS+s7DBlfKEu3RzzvZTaX1JtYbX4+AVUqj6ARz8IM+CKByqGFvbHN/\nW64XxNI+q7z36ajCL3VTJ2W5G9MCJitc6oGbire4NQfhaEq0nC+hxBWQvCbIFflA\n2ItrLUbSU7z1bHA/a+jlQm4OWvY+IKnTryOJTPuT1yNOVjbJ8wBLKy2DgQr9pPqW\nPmEQtGpR5IM9V8Kao6PaSdKYOWGbX3i2+RzIKhvZUxxtJwdVbqPlDPlW9h4/xIBc\n56Lgvr4aIl8nFtwbj4UJVUTFuGrs0tY9H/tXvZ22dUCKuGxW/gW7ZF+gXz6vHtYa\nrQIDAQAB\n-----END PUBLIC KEY-----', padding: crypto.constants.RSA_PKCS1_OAEP_PADDING, oaepHash: 'sha256' }, Buffer.from(key, 'utf8')).toString('base64');
    return { edt, sid };
}

// ========== CONFIG ==========
const APPID = '3116', DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = '4b0a5cb94103098b612eecd6f9d4cc08', TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990';
const HASH = '3970e49f52b3097d1b477cc35ed7da46';
const LITE_PEM = `-----BEGIN PUBLIC KEY-----\nMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDECi0Np2UR87scwrvTr72L6oO01rBbbBPriSDFPxr3Z5syug0O24QyQO8bg27+0+4kBzTBTBOZ/WWU0WryL1JSXRTXLgFVxtzIY41Pe7lPOgsfTCn5kZcvKhYKJesKnnJDNr5/abvTGf+rHG3YRwsCHcQ08/q6ifSioBszvb3QiwIDAQAB\n-----END PUBLIC KEY-----`;

function reqHeaders(t) {
    return {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'Content-Type': 'application/json',
        dfid: DFID, mid: MID, clienttime: String(t),
        'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
    };
}

async function main() {
    // Step 0: v5 playback trigger (same as test_v5_moekoe)
    console.log('=== Step 0: v5 playback ===');
    const ct0 = Math.floor(Date.now() / 1000);
    const v5params = {
        album_id: 0, area_code: 1, hash: HASH, ssa_flag: 'is_fromtrack',
        version: 11430, page_id: 967177915, quality: 128, album_audio_id: 0,
        behavior: 'play', pid: 411, cmd: 26, pidversion: 3001, IsFreePart: 0,
        ppage_id: '356753938,823673182,967485191', cdnBackup: 1, module: '',
        clientver: 11430, appid: APPID, dfid: DFID, mid: MID, uuid: '-',
        clienttime: ct0, token: TOKEN, userid: USERID,
    };
    v5params.key = signKey(HASH, MID, USERID, APPID);
    v5params.signature = sigAndroid(v5params, '');
    const qs0 = Object.keys(v5params).map(k => `${k}=${encodeURIComponent(v5params[k])}`).join('&');

    const r0 = await httpFull('GET', 'gateway.kugou.com', '/v5/url?' + qs0, {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'x-router': 'trackercdn.kugou.com',
        dfid: DFID, mid: MID, clienttime: String(ct0),
        'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
    });

    const ssaCode = r0.headers['ssa-code'];
    console.log('v5 errcode:', r0.json?.errcode, 'ssa-code:', ssaCode);
    if (!ssaCode) { console.log('❌ No ssa-code'); return; }

    // Step 1: get_verify_info
    console.log('\n=== Step 1: get_verify_info ===');
    const ct1 = Math.floor(Date.now() / 1000);
    const p1 = { appid: APPID, clientver: '11430', clienttime: ct1, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    // Step 2: generate sid/edt BEFORE step 1 body
    const webglHash = md5('webgl_' + randomString(16));
    const { edt, sid } = generateSimulate(MID, USERID, DFID, webglHash);

    const body1 = JSON.stringify({ eventid: ssaCode, userid: Number(USERID), platid: 2, rtype: 1, wasm: 1, i: '', sid: encodeURIComponent(sid), edt: encodeURIComponent(edt) });
    const qs1 = Object.keys(p1).sort().map(k => `${k}=${p1[k]}`).join('&') + '&signature=' + sigAndroid(p1, body1);
    const r1 = await httpFull('POST', 'gateway.kugou.com', '/verifyservice/v3/get_verify_info?' + qs1, reqHeaders(ct1), body1);
    console.log('get_verify_info:', JSON.stringify(r1.json, null, 2));
    if (r1.json?.status !== 1) { console.log('❌ Step 1 failed'); return; }
    const vType = r1.json.data.v_type;
    console.log('v_type:', vType);

    // Step 2: generate sid/edt
    console.log('\n=== Step 2: generate sid/edt ===');
    const webglHash = md5('webgl_' + randomString(16));
    const { edt, sid } = generateSimulate(MID, USERID, DFID, webglHash);
    console.log('edt[..50]:', edt.slice(0,50), 'sid[..50]:', sid.slice(0,50));

    // Step 3: verify_user_info
    console.log('\n=== Step 3: verify_user_info ===');
    const tempKey = randomString(16).toLowerCase();
    const aesKey = md5(tempKey).substring(0, 32);
    const aesIv = aesKey.substring(aesKey.length - 16);
    const paramsHex = aesEncrypt('{}', aesKey, aesIv);
    const pkHex = rsaRaw(JSON.stringify({ key: tempKey }), LITE_PEM);

    const ct3 = Math.floor(Date.now() / 1000);
    const p3 = { appid: APPID, clientver: '11510', clienttime: ct3, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    const vBody = JSON.stringify({
        eventid: ssaCode, userid: Number(USERID), platid: 2,
        v_type: vType, wasm: 1, i: '',
        sid: encodeURIComponent(sid),
        edt: encodeURIComponent(edt),
        verifycode: '', pk: pkHex, params: paramsHex,
    });
    const qs3 = Object.keys(p3).sort().map(k => `${k}=${p3[k]}`).join('&') + '&signature=' + sigAndroid(p3, vBody);
    const r3 = await httpFull('POST', 'verifyservice.kugou.com', '/v4/verify_user_info?' + qs3, reqHeaders(ct3), vBody);
    console.log('verify_user_info:', JSON.stringify(r3.json, null, 2));

    if (r3.json?.status === 1) {
        console.log('\n✅ SSA 验证通过！v5 重试应该能拿到 URL');
    } else if (r3.json?.error_code === 36010) {
        console.log('\n⚠️  需要滑块验证码 (v_type=' + vType + ', error_code=36010)');
    } else {
        console.log('\n⚠️  其他错误: ' + JSON.stringify(r3.json));
    }
}

main().catch(e => console.error('FATAL:', e));
