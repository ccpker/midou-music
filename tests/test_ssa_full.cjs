// 完整 SSA 验证流程（纯 Node 内置 crypto，零依赖）
const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }

function smd5be(bytes) {
    const h = crypto.createHash('md5');
    h.update(bytes);
    return h.digest('hex').substring(0, 16);
}

function randomString(len) {
    return crypto.randomBytes(Math.ceil(len/2)).toString('hex').slice(0, len);
}

// --- 签名 ---
function sigAndroid(params, body) {
    const salt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA'; // lite
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => `${k}=${typeof params[k] === 'object' ? JSON.stringify(params[k]) : params[k]}`).join('');
    return md5(salt + kv + (body || '') + salt);
}

// --- HTTP helpers ---
function httpReq(method, hostname, path, headers, body) {
    return new Promise((resolve, reject) => {
        const opts = { hostname, path, method, headers };
        const req = https.request(opts, res => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => {
                const h = {};
                Object.entries(res.headers).forEach(([k,v]) => h[k] = v);
                try { resolve({ status: res.statusCode, headers: h, body: JSON.parse(data) }); }
                catch { resolve({ status: res.statusCode, headers: h, body: data }); }
            });
        });
        req.on('error', reject);
        if (body) req.write(body);
        req.end();
    });
}

// --- AES-128-CBC + RSA-OAEP (generate_simulate 核心) ---
function generateSimulate(mid, userid, dfid, webglHash) {
    const SENTINEL = 0xffffffff - Math.floor(Math.random() * 20);
    const key = smd5be(Buffer.from(randomString(16), 'utf8'));

    // 鼠标轨迹 etc.
    const ri = (min, max) => Math.floor(Math.random() * (max - min + 1)) + min;
    const f3 = (t, i, x, y) => `3,${t},${i},${x},${y}`;
    const f5 = (t, i) => `5,${t},${i}`;
    const f6 = (t, i, x, y) => `6,${t},${i},${x},${y}`;
    const fs3 = (i, x, y) => `3,${SENTINEL},${i},${x},${y}`;
    const fs5 = (i) => `5,${SENTINEL},${i}`;
    const fs6 = (i, x, y) => `6,${SENTINEL},${i},${x},${y}`;

    let entries = [], ts = 0, ei = 0;
    entries.push(f5(0,0), fs5(0), f5(0,0), fs5(0));
    ts += ri(5,20);
    entries.push(f6(ts, ei, 750, 500), fs6(ei, 750, 500));
    ei++;

    for (let i = 0; i < 3; i++) {
        ts += ri(80, 600);
        entries.push(f5(ts, ei), fs5(ei));
        ei++;
    }

    const points = ri(30, 60);
    let sx = ri(200, 600), sy = ri(200, 500), ex = ri(500, 700), ey = ri(80, 150);
    const c1x = sx + (ex - sx) * 0.3 + ri(-80, 80);
    const c1y = sy + (ey - sy) * 0.2 + ri(-60, 60);
    const c2x = sx + (ex - sx) * 0.7 + ri(-60, 60);
    const c2y = sy + (ey - sy) * 0.8 + ri(-40, 40);

    let si = 0;
    for (let i = 0; i <= points; i++) {
        const t = i / points, u = 1 - t;
        let x = u*u*u*sx + 3*u*u*t*c1x + 3*u*t*t*c2x + t*t*t*ex;
        let y = u*u*u*sy + 3*u*u*t*c1y + 3*u*t*t*c2y + t*t*t*ey;
        const jitter = Math.max(0.5, 3 - t * 2.5);
        x += (Math.random() - 0.5) * jitter;
        y += (Math.random() - 0.5) * jitter;
        ts += ri(8, 50);
        entries.push(f3(ts, si, Math.round(x), Math.round(y)));
        entries.push(fs3(si, Math.round(x), Math.round(y)));
        if (i > 0 && i % 12 === 0) {
            ts += ri(20, 60);
            entries.push(f5(ts, ei), fs5(ei));
            ei++;
        }
        si = (si + 1) % 2;
    }
    ts += ri(5, 30);
    entries.push(f3(ts, 1, Math.round(ex + ri(-5,5)), Math.round(ey + ri(-5,5))));
    entries.push(fs3(1, Math.round(ex), Math.round(ey)));

    const data = entries.join(':');
    const plaintext = `mid=${mid || 0};userid=${userid || 0};dfid=${dfid || 0};webgl=${webglHash || generateWebGLHash()};webdriver=0;ts=${Date.now()};data=${data}`;

    // AES-128-CBC
    const iv = 'kugousecurity123';
    const cipher = crypto.createCipheriv('aes-128-cbc', Buffer.from(key, 'utf8'), Buffer.from(iv, 'utf8'));
    const edt = Buffer.concat([cipher.update(plaintext, 'utf8'), cipher.final()]).toString('base64');

    // RSA-OAEP SHA-256
    const pubPem = '-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoW2+Ylo8ALePSQTP0xBF\nlFmEOHvBD9tS+s7DBlfKEu3RzzvZTaX1JtYbX4+AVUqj6ARz8IM+CKByqGFvbHN/\nW64XxNI+q7z36ajCL3VTJ2W5G9MCJitc6oGbire4NQfhaEq0nC+hxBWQvCbIFflA\n2ItrLUbSU7z1bHA/a+jlQm4OWvY+IKnTryOJTPuT1yNOVjbJ8wBLKy2DgQr9pPqW\nPmEQtGpR5IM9V8Kao6PaSdKYOWGbX3i2+RzIKhvZUxxtJwdVbqPlDPlW9h4/xIBc\n56Lgvr4aIl8nFtwbj4UJVUTFuGrs0tY9H/tXvZ22dUCKuGxW/gW7ZF+gXz6vHtYa\nrQIDAQAB\n-----END PUBLIC KEY-----';
    const sid = crypto.publicEncrypt({
        key: pubPem,
        padding: crypto.constants.RSA_PKCS1_OAEP_PADDING,
        oaepHash: 'sha256',
    }, Buffer.from(key, 'utf8')).toString('base64');

    return { edt, sid };
}

function generateWebGLHash() {
    return md5('webgl_' + randomString(16));
}

// --- 主流程 ---
const APPID = '3116', DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = '4b0a5cb94103098b612eecd6f9d4cc08', TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990', WEBGL = generateWebGLHash();

async function main() {
    // Step A: 拿到 ssa-code（从来玩请求的响应头提取）
    // 我们先手动传 — 之前 v5 返回的 ssa-code
    const SSA_CODE = 'bj_tx_event_adb92cbd5c3dc2cb75bb38f717676529';

    // Step 1: get_verify_info
    console.log('=== Step 1: get_verify_info ===');
    const dataMap1 = JSON.stringify({
        eventid: SSA_CODE,
        userid: Number(USERID),
        platid: 2,
        rtype: 1,
        wasm: 1,
        i: '',
        sid: '',
        edt: '',
    });
    const t1 = Math.floor(Date.now() / 1000);
    const p1 = { appid: APPID, clientver: '11430', clienttime: t1, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    const qs1 = Object.keys(p1).sort().map(k => `${k}=${p1[k]}`).join('&') + '&signature=' + sigAndroid(p1, dataMap1);
    // get_verify_info baseURL 是默认的 gateway.kugou.com（源码没有设置 baseURL）
    const r1 = await httpReq('POST', 'gateway.kugou.com', '/verifyservice/v3/get_verify_info?' + qs1, {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'Content-Type': 'application/json',
        dfid: DFID, mid: MID, clienttime: String(t1),
        'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
    }, dataMap1);
    console.log('status:', r1.status);
    console.log('body:', JSON.stringify(r1.body, null, 2));

    if (r1.body.status !== 1) {
        console.log('❌ get_verify_info 失败');
        return;
    }

    const vType = r1.body.data?.v_type || 23;
    const txAppId = r1.body.data?.txappid || '';
    console.log('v_type:', vType, 'txappid:', txAppId);

    // Step 2: 生成 sid/edt
    console.log('\n=== Step 2: 生成 sid/edt ===');
    const { edt, sid } = generateSimulate(MID, USERID, DFID, WEBGL);
    console.log('edt[..60]:', edt.slice(0, 60));
    console.log('sid[..60]:', sid.slice(0, 60));

    // Step 3: verify_user_info（不带 captcha — v_type=23 不带验证码能过吗？）
    console.log('\n=== Step 3: verify_user_info ===');
    const dataMap3 = JSON.stringify({
        eventid: SSA_CODE,
        userid: Number(USERID),
        platid: 2,
        v_type: vType,
        wasm: 1,
        i: '',
        sid: sid,
        edt: edt,
        // v_type=23 需要 verifycode/pk/params — 不传试试
    });
    const t3 = Math.floor(Date.now() / 1000);
    // verify_user_info 的 baseURL 不是 verifyservice.kugou.com 加前缀？看源码是 baseURL: 'https://verifyservice.kugou.com', url: '/v4/verify_user_info'
    // 所以是 https://verifyservice.kugou.com/v4/verify_user_info
    const p3 = { appid: APPID, clientver: '11510', clienttime: t3, dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID };
    const qs3 = Object.keys(p3).sort().map(k => `${k}=${p3[k]}`).join('&') + '&signature=' + sigAndroid(p3, dataMap3);
    // verify_user_info 走 verifyservice.kugou.com（源码 baseURL）
    const r3 = await httpReq('POST', 'verifyservice.kugou.com', '/v4/verify_user_info?' + qs3, {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'Content-Type': 'application/json',
        dfid: DFID, mid: MID, clienttime: String(t3),
        'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
    }, dataMap3);
    console.log('status:', r3.status);
    console.log('body:', JSON.stringify(r3.body, null, 2));

    // Step 4: 重试 v5
    if (r3.body.status === 1) {
        console.log('\n=== Step 4: 重试 v5 播放 ===');
        // 直接用之前的 test_v5_moekoe 重试
    }
}

main().catch(e => console.error('FATAL:', e));
