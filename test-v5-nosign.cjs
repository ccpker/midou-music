// test-v5-nosign.cjs — 测试 v5 含 clienttime
const crypto = require('crypto');
const http = require('http');

const HASH = 'B3A52A7A958BF0AED0EBFBA2E9A818B7';
const dfid = '2a1b2c3d4e5f6a7b8c9d0e1f';

function md5Hex(s) { return crypto.createHash('md5').update(s).digest('hex'); }
const mid = md5Hex(dfid);

function doRequest(label, path, body) {
    const bodyJson = JSON.stringify(body);
    const headers = {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'dfid': dfid,
        'mid': mid,
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(bodyJson)
    };
    console.log(`\n=== ${label} ===`);

    return new Promise((resolve) => {
        const req = http.request({ hostname: 'trackercdn.kugou.com', port: 80, path, method: 'POST', headers }, (res) => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => {
                try {
                    const j = JSON.parse(data);
                    if (j.errcode === 0 && j.url) {
                        console.log('✅ errcode=0, url len:', j.url.length);
                    } else {
                        console.log('❌ errcode:', j.errcode, '| msg:', j.errmsg || j.msg);
                    }
                } catch(e) { console.log('Raw:', data.slice(0, 300)); }
                resolve();
            });
        });
        req.on('error', e => { console.error('Error:', e.message); resolve(); });
        req.write(bodyJson);
        req.end();
    });
}

async function main() {
    const ts = Math.floor(Date.now() / 1000);
    const tsm = Date.now();

    // 测试1: 无签名无clienttime（baseline）
    await doRequest('v5无签名无clienttime', '/v5/url', {
        appid: 1005, clientver: 20489, dfid, hash: HASH, mid, notSign: true
    });
    await new Promise(r => setTimeout(r, 400));

    // 测试2: 加 clienttime 秒
    await doRequest('v5无签名+clienttime(秒)', '/v5/url', {
        appid: 1005, clientver: 20489, dfid, hash: HASH, mid, clienttime: ts, notSign: true
    });
    await new Promise(r => setTimeout(r, 400));

    // 测试3: 加 clienttime 毫秒
    await doRequest('v5无签名+clienttime(毫秒)', '/v5/url', {
        appid: 1005, clientver: 20489, dfid, hash: HASH, mid, clienttime: tsm, notSign: true
    });
    await new Promise(r => setTimeout(r, 400));

    // 测试4: 加 signKey（encryptKey=true）
    const signKey = crypto.createHash('md5')
        .update(HASH + '57ae12eb6890223e355ccfcb74edf70d' + '1005' + mid + '0')
        .digest('hex');
    await doRequest('v5无签名+signKey', '/v5/url', {
        appid: 1005, clientver: 20489, dfid, hash: HASH, mid, clienttime: ts,
        notSign: true, key: signKey
    });
    await new Promise(r => setTimeout(r, 400));

    // 测试5: 标准版盐值 (57ae12eb...) vs 概念版盐值 (185672dd...)
    // signKey = MD5(hash + SIGNKEY_SALT + appid + mid + userid)
    const sk1 = md5Hex(HASH + '57ae12eb6890223e355ccfcb74edf70d' + '1005' + mid + '0'); // 标准版
    const sk2 = md5Hex(HASH + '185672dd44712f60bb1736df5a377e82' + '3116' + mid + '0'); // 概念版
    console.log('\n=== v5 signKey 测试 ===');
    console.log('标准版 signKey:', sk1);
    console.log('概念版 signKey:', sk2);
    await doRequest('v5+signKey(标准版salt)', '/v5/url', {
        appid: 1005, clientver: 20489, dfid, hash: HASH, mid, clienttime: ts,
        notSign: true, key: sk1
    });
}

main().then(() => console.log('\nAll done'));
