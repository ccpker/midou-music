// SSA 修复：sid/edt 放到 query string（不是 JSON body）
const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }
function smd5be(bytes) { const h = crypto.createHash('md5'); h.update(bytes); return h.digest('hex').substring(0, 16); }
function randomString(len) { const r = crypto.randomBytes(Math.ceil(len/2)).toString('hex').slice(0, len); return r; }

const SALT = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
const ENC_SALT = '185672dd44712f60bb1736df5a377e82';
const OAEP_PEM = '-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoW2+Ylo8ALePSQTP0xBF\nlFmEOHvBD9tS+s7DBlfKEu3RzzvZTaX1JtYbX4+AVUqj6ARz8IM+CKByqGFvbHN/\nW64XxNI+q7z36ajCL3VTJ2W5G9MCJitc6oGbire4NQfhaEq0nC+hxBWQvCbIFflA\n2ItrLUbSU7z1bHA/a+jlQm4OWvY+IKnTryOJTPuT1yNOVjbJ8wBLKy2DgQr9pPqW\nPmEQtGpR5IM9V8Kao6PaSdKYOWGbX3i2+RzIKhvZUxxtJwdVbqPlDPlW9h4/xIBc\n56Lgvr4aIl8nFtwbj4UJVUTFuGrs0tY9H/tXvZ22dUCKuGxW/gW7ZF+gXz6vHtYa\nrQIDAQAB\n-----END PUBLIC KEY-----';
const LITE_PEM = '-----BEGIN PUBLIC KEY-----\nMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDECi0Np2UR87scwrvTr72L6oO01rBbbBPriSDFPxr3Z5syug0O24QyQO8bg27+0+4kBzTBTBOZ/WWU0WryL1JSXRTXLgFVxtzIY41Pe7lPOgsfTCn5kZcvKhYKJesKnnJDNr5/abvTGf+rHG3YRwsCHcQ08/q6ifSioBszvb3QiwIDAQAB\n-----END PUBLIC KEY-----';

function signKey(hash, mid, uid, appid) { return md5(`${hash}${ENC_SALT}${appid}${mid}${uid||0}`); }
function sigAndroid(params, body) {
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => `${k}=${typeof params[k] === 'object' ? JSON.stringify(params[k]) : params[k]}`).join('');
    return md5(`${SALT}${kv}${body||''}${SALT}`);
}

function httpFull(method, hostname, path, headers, body) {
    return new Promise((resolve, reject) => {
        const req = https.request({ hostname, path, method, headers, rejectUnauthorized: false }, res => {
            let d = ''; res.on('data', c => d += c);
            res.on('end', () => { resolve({ status: res.statusCode, headers: Object.fromEntries(Object.entries(res.headers)), body: d, json: (() => { try { return JSON.parse(d); } catch { return null; } })() }); });
        });
        req.on('error', reject);
        if (body) req.write(body);
        req.end();
    });
}

// AES-128-CBC for edt
function aes128Encrypt(plain, keyHex) {
    const c = crypto.createCipheriv('aes-128-cbc', Buffer.from(keyHex,'utf8'), Buffer.from('kugousecurity123','utf8'));
    return Buffer.concat([c.update(plain,'utf8'), c.final()]).toString('base64');
}

// RSA-OAEP for sid
function rsaOaepEncrypt(data) {
    return crypto.publicEncrypt({ key: OAEP_PEM, padding: crypto.constants.RSA_PKCS1_OAEP_PADDING, oaepHash: 'sha256' }, Buffer.from(data,'utf8')).toString('base64');
}

// AES-256-CBC for pk/params
function aes256Encrypt(data, key, iv) {
    const c = crypto.createCipheriv('aes-256-cbc', Buffer.from(key,'utf8'), Buffer.from(iv,'utf8'));
    return Buffer.concat([c.update(data,'utf8'), c.final()]).toString('hex');
}

// RSA raw for pk
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

// generateSimulate
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
    const edt = aes128Encrypt(plain, key);
    const sid = rsaOaepEncrypt(key);
    return { edt, sid };
}

// ========== CONFIG ==========
const APPID = '3116', DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = '4b0a5cb94103098b612eecd6f9d4cc08', TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990', HASH = '3970e49f52b3097d1b477cc35ed7da46';

function reqHeaders(t) {
    return { 'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi', 'Content-Type': 'application/json', dfid: DFID, mid: MID, clienttime: String(t), 'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F' };
}

async function main() {
    // Step 0: v5 trigger
    console.log('=== Step 0: v5 trigger ===');
    const ct0 = Math.floor(Date.now() / 1000);
    const v5params = { album_id:0, area_code:1, hash:HASH, ssa_flag:'is_fromtrack', version:11430, page_id:967177915, quality:128, album_audio_id:0, behavior:'play', pid:411, cmd:26, pidversion:3001, IsFreePart:0, ppage_id:'356753938,823673182,967485191', cdnBackup:1, module:'', clientver:11430, appid:APPID, dfid:DFID, mid:MID, uuid:'-', clienttime:ct0, token:TOKEN, userid:USERID };
    v5params.key = signKey(HASH, MID, USERID, APPID);
    v5params.signature = sigAndroid(v5params, '');
    const qs0 = Object.keys(v5params).map(k => `${k}=${encodeURIComponent(v5params[k])}`).join('&');
    const r0 = await httpFull('GET', 'gateway.kugou.com', '/v5/url?' + qs0, {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi', 'x-router': 'trackercdn.kugou.com',
        dfid: DFID, mid: MID, clienttime: String(ct0), 'kg-rc':'1','kg-thash':'5d816a0','kg-rec':'1','kg-rf':'B9EDA08A64250DEFFBCADDEE00F8F25F',
    });
    const ssaCode = r0.headers['ssa-code'];
    console.log('errcode:', r0.json?.errcode, 'ssa:', ssaCode);
    if (!ssaCode) { console.log('No ssa-code'); return; }

    // Generate fingerprint once
    const webglHash = md5('webgl_' + randomString(16));
    const { edt, sid } = generateSimulate(MID, USERID, DFID, webglHash);
    console.log('edt[..50]:', edt.slice(0,50), '\nsid[..50]:', sid.slice(0,50));

    // ★ Try 1: sid/edt in JSON body (no encode)
    console.log('\n=== Try 1: JSON body, raw base64 ===');
    const ct1 = Math.floor(Date.now() / 1000);
    const p1 = { appid: APPID, clientver: '11430', clienttime: ct1, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    const body1 = JSON.stringify({ eventid: ssaCode, userid: Number(USERID), platid: 2, rtype: 1, wasm: 1, i: '', sid: sid, edt: edt });
    const qs1 = Object.keys(p1).sort().map(k => `${k}=${p1[k]}`).join('&') + '&signature=' + sigAndroid(p1, body1);
    const r1 = await httpFull('POST', 'gateway.kugou.com', '/verifyservice/v3/get_verify_info?' + qs1, reqHeaders(ct1), body1);
    console.log('r1:', JSON.stringify(r1.json));
    if (r1.json?.status !== 1) { console.log('Try 1 failed'); return; }
    const vType = r1.json.data.v_type;
    console.log('v_type:', vType);

    // ★ Try 2: verify_user_info — sid/edt in query params
    console.log('\n=== Try 2: verify_user_info (sid/edt in URL) ===');
    const tempKey = randomString(16).toLowerCase();
    const aesKey = md5(tempKey).substring(0, 32);
    const aesIv = aesKey.substring(aesKey.length - 16);
    const paramsHex = aes256Encrypt('{}', aesKey, aesIv);
    const pkHex = rsaRaw(JSON.stringify({ key: tempKey }), LITE_PEM);

    const ct3 = Math.floor(Date.now() / 1000);
    // sid/edt in query params
    const p3full = { appid: APPID, clientver: '11510', clienttime: ct3, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID, eventid: ssaCode, platid: '2', v_type: '23', wasm: '1', sid: encodeURIComponent(sid), edt: encodeURIComponent(edt), verifycode: '', pk: pkHex, params: paramsHex };
    const vBody = JSON.stringify({});
    const qs3 = Object.keys(p3full).sort().map(k => `${k}=${p3full[k]}`).join('&') + '&signature=' + sigAndroid(p3full, '');
    const r3 = await httpFull('POST', 'verifyservice.kugou.com', '/v4/verify_user_info?' + qs3, reqHeaders(ct3), vBody);
    console.log(JSON.stringify(r3.json, null, 2));

    // ★ Try 3: verify_user_info — sid/edt in JSON body (raw, no encode)
    console.log('\n=== Try 3: verify_user_info (sid/edt in JSON body, raw) ===');
    const ct4 = Math.floor(Date.now() / 1000);
    const { edt: edt2, sid: sid2 } = generateSimulate(MID, USERID, DFID, webglHash);
    const tk2 = randomString(16).toLowerCase();
    const ak2 = md5(tk2).substring(0, 32), iv2 = ak2.substring(ak2.length - 16);
    const p2Hex = aes256Encrypt('{}', ak2, iv2);
    const pk2Hex = rsaRaw(JSON.stringify({ key: tk2 }), LITE_PEM);
    const p4 = { appid: APPID, clientver: '11510', clienttime: ct4, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    const vBody3 = JSON.stringify({ eventid: ssaCode, userid: Number(USERID), platid: 2, v_type: vType, wasm: 1, i: '', sid: sid2, edt: edt2, verifycode: '', pk: pk2Hex, params: p2Hex });
    const qs4 = Object.keys(p4).sort().map(k => `${k}=${p4[k]}`).join('&') + '&signature=' + sigAndroid(p4, vBody3);
    const r4 = await httpFull('POST', 'verifyservice.kugou.com', '/v4/verify_user_info?' + qs4, reqHeaders(ct4), vBody3);
    console.log(JSON.stringify(r4.json, null, 2));

    // ★ Try 4: 不传 pk/params，让服务器自己算（v_type=23 is text verification, might accept without captcha if we're lucky）
    console.log('\n=== Try 4: no pk/params ===');
    const ct5 = Math.floor(Date.now() / 1000);
    const { edt: edt3, sid: sid3 } = generateSimulate(MID, USERID, DFID, webglHash);
    const p5 = { appid: APPID, clientver: '11510', clienttime: ct5, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    const vBody4 = JSON.stringify({ eventid: ssaCode, userid: Number(USERID), platid: 2, v_type: vType, wasm: 1, i: '', sid: sid3, edt: edt3 });
    const qs5 = Object.keys(p5).sort().map(k => `${k}=${p5[k]}`).join('&') + '&signature=' + sigAndroid(p5, vBody4);
    const r5 = await httpFull('POST', 'verifyservice.kugou.com', '/v4/verify_user_info?' + qs5, reqHeaders(ct5), vBody4);
    console.log(JSON.stringify(r5.json, null, 2));
}

main().catch(e => console.error('FATAL:', e));
