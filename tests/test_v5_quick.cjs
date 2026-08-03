const crypto = require('crypto');
const https = require('https');

const DFID='2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6',MID='4b0a5cb94103098b612eecd6f9d4cc08';
const TOKEN='91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239',USERID='1514557990';
const ct=Math.floor(Date.now()/1000);

// 不对完整的 album_audio_id 签名（test_v5_moekoe 签名范式：只按字母排序 params）
// 直接复用 test_v5_moekoe.cjs 的签名逻辑
const SHARED_KEY = 'NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt';
function sigV5(params) {
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => k + '=' + params[k]).join('&');
    return crypto.createHash('md5').update(SHARED_KEY + kv + SHARED_KEY).digest('hex');
}

const t = Math.floor(Date.now() / 1000);
const v5params = {
    appid: '3116',
    album_audio_id: '218822496',
    album_id: '69574375',
    clientver: '11430',
    clienttime: t,
    dfid: DFID,
    isRetry: 'true',
    mid: MID,
    token: TOKEN,
    userid: USERID,
    uuid: '-',
};

const sigV5Val = sigV5(v5params);
const qs = Object.keys(v5params).sort().map(k => k + '=' + v5params[k]).join('&') + '&signature=' + sigV5Val;

console.log('GET /v5/url?' + qs);

const req = https.request({
    hostname: 'gateway.kugou.com',
    path: '/v5/url?' + qs,
    method: 'GET',
    headers: {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'x-router': 'trackercdn',
        dfid: DFID,
        mid: MID,
    },
}, res => {
    let d = '';
    res.on('data', c => d += c);
    res.on('end', () => {
        console.log('STATUS:', res.statusCode);
        Object.entries(res.headers).forEach(([k,v]) => console.log(k + ':', v));
        console.log('BODY:', d.slice(0, 500));
    });
});
req.on('error', e => console.error('ERR:', e));
req.end();
