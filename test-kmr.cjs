// test-kmr.cjs — 测试 kmr.service.kugou.com/v1/audio/audio
// 完全复刻 audio.js + helper.js + config.json
const crypto = require('crypto');
const http = require('http');

const HASH = 'B3A52A7A958BF0AED0EBFBA2E9A818B7';

// === helper.js ===
const SIGNPARAMSKEY_SALT = 'OIlwieks28dk2k092lksi2UIkp';
const SIGN_ANDROID_SALT = 'OIlwieks28dk2k092lksi2UIkp';
const APPID = '2';
const CLIENTVER = '1210';

function signParamsKey(data, appid, clientver) {
    const isLite = false;
    const str = SIGNPARAMSKEY_SALT;
    appid = appid || (isLite ? '3116' : APPID);
    clientver = clientver || (isLite ? '11440' : CLIENTVER);
    return crypto.createHash('md5').update(`${appid}${str}${clientver}${data}`).digest('hex');
}

function signatureAndroidParams(params, data) {
    const str = SIGN_ANDROID_SALT;
    const paramsString = Object.keys(params).sort()
        .map(k => `${k}=${typeof params[k] === 'object' ? JSON.stringify(params[k]) : params[k]}`)
        .join('');
    if (Buffer.isBuffer(data)) {
        // Binary body — use CryptoJS-like approach with raw MD5
        const h = crypto.createHash('md5');
        h.update(str); h.update(paramsString); h.update(data); h.update(str);
        return h.digest('hex');
    }
    return crypto.createHash('md5').update(`${str}${paramsString}${data||''}${str}`).digest('hex');
}

function signKey(hash, mid, userid, appid) {
    const isLite = false;
    const str = isLite ? '185672dd44712f60bb1736df5a377e82' : '57ae12eb6890223e355ccfcb74edf70d';
    return crypto.createHash('md5').update(`${hash}${str}${appid||'2'}${mid||'0'}${userid||0}`).digest('hex');
}

// === 测试 ===
const DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const dateTime = Math.floor(Date.now()); // ms for kmr

async function test(label, host, port, path, method, body, extraHeaders) {
    const bodyJson = JSON.stringify(body);
    const headers = {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(bodyJson),
        'x-router': host,
        ...extraHeaders
    };
    console.log(`\n=== ${label} ===`);
    console.log('POST', `${host}:${port}${path}`);

    return new Promise(resolve => {
        const req = http.request({ hostname: host, port, path, method, headers }, res => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => {
                try {
                    const j = JSON.parse(data);
                    if (j.errcode === 0) {
                        console.log('✅ errcode=0');
                        if (j.data && j.data[0]) console.log('url:', j.data[0].url ? j.data[0].url.slice(0,80)+'...' : 'null');
                        else if (j.url) console.log('url:', j.url.slice(0,80)+'...');
                        else console.log('data:', JSON.stringify(j.data||j).slice(0,200));
                    } else {
                        console.log('❌ errcode:', j.errcode, '| msg:', j.errmsg || j.msg);
                        if (j.data) console.log('data preview:', JSON.stringify(j.data).slice(0,200));
                    }
                } catch(e) { console.log('Raw:', data.slice(0,300)); }
                resolve();
            });
        });
        req.on('error', e => { console.log('Error:', e.message); resolve(); });
        req.write(bodyJson);
        req.end();
    });
}

async function main() {
    // === kmr.service.kugou.com/v1/audio/audio ===
    // audio.js 格式（标准版，无mid）
    const kmrBody1 = {
        appid: APPID,
        clienttime: dateTime,
        clientver: CLIENTVER,
        data: [{ hash: HASH, audio_id: 0 }],
        dfid: DFID,
        key: signParamsKey(dateTime.toString()),
    };
    await test('kmr v1 标准版 (无mid/无加密)', 'kmr.service.kugou.com', 80, '/v1/audio/audio', 'POST', kmrBody1, {});
    await new Promise(r => setTimeout(r, 500));

    // 加 encryptType=android 签名
    const kmrParams = {
        appid: APPID,
        clienttime: dateTime,
        clientver: CLIENTVER,
        dfid: DFID,
        data: JSON.stringify(kmrBody1.data),
        key: signParamsKey(dateTime.toString()),
    };
    kmrParams['signature'] = signatureAndroidParams(kmrParams, kmrBody1.data);

    const kmrBody2 = {
        appid: APPID,
        clienttime: dateTime,
        clientver: CLIENTVER,
        data: [{ hash: HASH, audio_id: 0 }],
        dfid: DFID,
        key: signParamsKey(dateTime.toString()),
        signature: kmrParams['signature'],
    };
    await test('kmr v1 标准版 + android签名', 'kmr.service.kugou.com', 80, '/v1/audio/audio', 'POST', kmrBody2, {});
    await new Promise(r => setTimeout(r, 500));

    // 用 KUGOU_API_MID=null（实际字符串'null'）
    const kmrBody3 = { ...kmrBody2, mid: 'null' };
    await test('kmr v1 标准版 mid=null', 'kmr.service.kugou.com', 80, '/v1/audio/audio', 'POST', kmrBody3, {});
    await new Promise(r => setTimeout(r, 500));

    // kmr lite 版 (platform='lite')
    const liteAppid = '3116', liteClientver = '11440';
    const liteParamsKey = crypto.createHash('md5')
        .update(`${liteAppid}LnT6xpN3khm36zse0QzvmgTZ3waWdRSA${liteClientver}${dateTime}`)
        .digest('hex');
    const kmrBody4 = {
        appid: liteAppid,
        clienttime: dateTime,
        clientver: liteClientver,
        data: [{ hash: HASH, audio_id: 0 }],
        dfid: DFID,
        key: liteParamsKey,
    };
    await test('kmr v1 lite版 (appid=3116)', 'kmr.service.kugou.com', 80, '/v1/audio/audio', 'POST', kmrBody4, {});
    await new Promise(r => setTimeout(r, 500));

    // kmr lite + android签名
    const liteParams = {
        appid: liteAppid,
        clienttime: dateTime,
        clientver: liteClientver,
        dfid: DFID,
        data: JSON.stringify(kmrBody4.data),
        key: liteParamsKey,
    };
    const liteSignSalt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
    const liteParamsString = Object.keys(liteParams).sort()
        .map(k => `${k}=${liteParams[k]}`).join('');
    const liteSig = crypto.createHash('md5')
        .update(`${liteSignSalt}${liteParamsString}${liteParams.data}${liteSignSalt}`)
        .digest('hex');
    const kmrBody5 = { ...kmrBody4, signature: liteSig };
    await test('kmr v1 lite版 + android签名', 'kmr.service.kugou.com', 80, '/v1/audio/audio', 'POST', kmrBody5, {});

    // === trackercdn.kugou.com/v5/url ===
    console.log('\n\n--- v5 CDN URL ---');
    const mid = '0'; // test mid=0
    const v5Params = {
        appid: '1005',
        clientver: '20489',
        dfid: DFID.slice(0,24),
        hash: HASH,
        mid: mid,
        clienttime: Math.floor(Date.now()/1000),
    };
    v5Params['key'] = signKey(v5Params.hash, v5Params.mid, '0', v5Params.appid);
    v5Params['signature'] = signatureAndroidParams(v5Params, '');

    const v5qs = Object.keys(v5Params).sort().map(k => `${k}=${encodeURIComponent(v5Params[k])}`).join('&');
    const v5Headers = {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'dfid': DFID.slice(0,24), 'mid': mid, 'clienttime': v5Params.clienttime,
        'x-router': 'trackercdn.kugou.com',
    };
    await test('v5 CDN mid=0 + android签名', 'trackercdn.kugou.com', 80, `/v5/url?${v5qs}`, 'GET', {}, v5Headers);
}

main().then(() => console.log('\n\nAll done'));
