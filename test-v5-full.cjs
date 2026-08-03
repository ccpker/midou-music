// test-v5-full.cjs — 完整签名验证测试
const crypto = require('crypto');
const http = require('http');

const HASH = 'B3A52A7A958BF0AED0EBFBA2E9A818B7';
const DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const APPID = '1005';
const CLIENTVER = '20489';
const SIGN_SALT = 'OIlwieks28dk2k092lksi2UIkp';
const SIGNKEY_SALT = '57ae12eb6890223e355ccfcb74edf70d';

function md5Hex(s) { return crypto.createHash('md5').update(s).digest('hex'); }

// KuGouMusicApi 里的 calculateMid 实现
function calculateMid(str) {
    let bigInteger = BigInt(0);
    const base = BigInt(16);
    const digest = crypto.createHash('md5').update(str).digest('hex');
    for (let i = 0; i < digest.length; i++) {
        const charValue = BigInt(parseInt(digest[i], 16));
        const powerValue = base ** BigInt(digest.length - 1 - i);
        bigInteger += charValue * powerValue;
    }
    return bigInteger.toString();
}

// 我的旧版 calculateMid（用 parseInt(hex, 16) 直接转换）
function calculateMidLegacy(str) {
    const digest = crypto.createHash('md5').update(str).digest('hex');
    return parseInt('0x' + digest, 16).toString();
}

// signatureAndroidParams（标准版）
function signatureAndroidParams(params, data) {
    const sorted = Object.keys(params).sort();
    const paramsString = sorted.map(k => `${k}=${params[k]}`).join('');
    return md5Hex(SIGN_SALT + paramsString + (data || '') + SIGN_SALT);
}

// signKey（标准版）
function signKey(hash, mid, userid, appid) {
    return md5Hex(hash + SIGNKEY_SALT + appid + mid + userid);
}

async function doRequest(label, body, midCalc) {
    const mid = midCalc(DFID);
    const clienttime = Math.floor(Date.now() / 1000);
    const dfid_match = DFID.match(/[\dA-Fa-f]{24}/);
    const dfid_24 = dfid_match ? dfid_match[0] : DFID;

    const params = {
        appid: APPID,
        clientver: CLIENTVER,
        dfid: dfid_24,
        hash: HASH,
        mid: mid,
        clienttime: clienttime,
        notSign: false,
    };
    params['key'] = signKey(params.hash, params.mid, '0', params.appid);
    params['signature'] = signatureAndroidParams(params, '');

    const bodyJson = JSON.stringify({ hash: HASH, ...body });
    const sorted = Object.keys(params).sort();
    const paramsString = sorted.map(k => `${k}=${params[k]}`).join('');

    console.log(`\n=== ${label} ===`);
    console.log('mid (first 20):', mid.slice(0, 20) + '...');
    console.log('dfid_24:', dfid_24);
    console.log('key:', params['key'].slice(0, 16) + '...');
    console.log('signature:', params['signature'].slice(0, 16) + '...');

    const queryStr = sorted.map(k => `${k}=${encodeURIComponent(params[k])}`).join('&');

    return new Promise(resolve => {
        const req = http.request(
            { hostname: 'trackercdn.kugou.com', port: 80, path: `/v5/url?${queryStr}`, method: 'GET',
              headers: {
                  'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
                  'dfid': dfid_24, 'mid': mid, 'clienttime': clienttime,
              }},
            res => {
                let data = '';
                res.on('data', c => data += c);
                res.on('end', () => {
                    try {
                        const j = JSON.parse(data);
                        console.log('errcode:', j.errcode, '| msg:', j.errmsg || j.msg);
                        if (j.errcode === 0) console.log('✅ URL:', j.url.slice(0, 80));
                    } catch(e) { console.log('Raw:', data.slice(0, 200)); }
                    resolve();
                });
            });
        req.on('error', e => { console.log('Error:', e.message); resolve(); });
        req.end();
    });
}

async function main() {
    // 测试1: 我的旧版 calculateMid (parseInt hex)
    await doRequest('v5 旧版 calculateMid (parseInt)', {}, calculateMidLegacy);
    await new Promise(r => setTimeout(r, 500));

    // 测试2: KuGouMusicApi 的 calculateMid (BigInt 逐位)
    await doRequest('v5 新版 calculateMid (BigInt)', {}, calculateMid);
    await new Promise(r => setTimeout(r, 500));

    // 测试3: 用 dfid 里提取的纯 24 字符
    const dfid24 = 'a1b2c3d4e5f6a7b8c9d0e1f2';
    await doRequest('v5 纯24字符 dfid', {}, (d) => calculateMid(dfid24));
}

main().then(() => console.log('\n\nDone'));
