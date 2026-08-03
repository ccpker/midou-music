// 测试 MoeKoeMusic SSA 验证流程
const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }

function signatureAndroidParams(params, body) {
    const salt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => `${k}=${typeof params[k] === 'object' ? JSON.stringify(params[k]) : params[k]}`).join('');
    return md5(`${salt}${kv}${body || ''}${salt}`);
}

function httpsGet(urlPath, params) {
    return new Promise((resolve, reject) => {
        const qs = Object.keys(params).map(k => `${k}=${encodeURIComponent(params[k])}`).join('&');
        const options = {
            hostname: 'gateway.kugou.com',
            path: urlPath + '?' + qs,
            method: 'GET',
            headers: {
                'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
                dfid: DFID, mid: MID, clienttime: String(params.clienttime),
                'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
            },
        };
        const req = https.request(options, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => {
                try { resolve(JSON.parse(data)); }
                catch { resolve(data); }
            });
        });
        req.on('error', reject);
        req.end();
    });
}

const APPID = '3116'; const DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = '4b0a5cb94103098b612eecd6f9d4cc08'; const TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990';
const SSA_CODE = 'bj_tx_event_adb92cbd5c3dc2cb75bb38f717676529';

async function main() {
    // Step 1: get verify info
    console.log('=== Step 1: get/verify/info ===');
    const t = Math.floor(Date.now() / 1000);
    const p1 = { eventid: SSA_CODE, appid: APPID, dfid: DFID, mid: MID, uuid: '-', clienttime: t, token: TOKEN, userid: USERID, clientver: '11430' };
    p1.signature = signatureAndroidParams(p1, '');
    const info = await httpsGet('/get/verify/info', p1);
    console.log(JSON.stringify(info, null, 2));
}

main().catch(e => console.error(e));
