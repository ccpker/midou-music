#!/usr/bin/env node
// 最小单元：直接抄 MoeKoeMusic signatureAndroidParams + encryptKey 逻辑
// 验证 SSA 20028 是否是签名问题

const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }

// --- MoeKoeMusic 签名（lite 模式）---
function signKey(hash, mid, userid, appid) {
    const salt = '185672dd44712f60bb1736df5a377e82'; // lite
    return md5(`${hash}${salt}${appid}${mid}${userid || 0}`);
}

function signatureAndroidParams(params, body) {
    const salt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA'; // lite
    const keys = Object.keys(params).sort();
    const kv = keys.map(k => `${k}=${typeof params[k] === 'object' ? JSON.stringify(params[k]) : params[k]}`).join('');
    return md5(`${salt}${kv}${body || ''}${salt}`);
}

// --- 用 MoeKoeMusic 方式发 v5 ---
const TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990';
const DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = '4b0a5cb94103098b612eecd6f9d4cc08';
const APPID = '3116';
const HASH = '3970e49f52b3097d1b477cc35ed7da46'; // 歌单第一首

const params = {
    album_id: 0,
    area_code: 1,
    hash: HASH,
    ssa_flag: 'is_fromtrack',
    version: 11430,
    page_id: 967177915,
    quality: 128,
    album_audio_id: 0,
    behavior: 'play',
    pid: 411,
    cmd: 26,
    pidversion: 3001,
    IsFreePart: 0,
    ppage_id: '356753938,823673182,967485191',
    cdnBackup: 1,
    module: '',
    clientver: 11430,
    appid: APPID,
    dfid: DFID,
    mid: MID,
    uuid: '-',
    clienttime: Math.floor(Date.now() / 1000),
    token: TOKEN,
    userid: USERID,
};

// encryptKey
params.key = signKey(HASH, MID, USERID, APPID);

// signature
const sig = signatureAndroidParams(params, '');
params.signature = sig;

console.log('signature:', sig);
console.log('key:', params.key);

const qs = Object.keys(params).map(k => `${k}=${encodeURIComponent(params[k])}`).join('&');

const options = {
    hostname: 'gateway.kugou.com',
    path: '/v5/url?' + qs,
    method: 'GET',
    headers: {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'x-router': 'trackercdn.kugou.com',
        dfid: DFID,
        mid: MID,
        clienttime: String(params.clienttime),
        'kg-rc': '1',
        'kg-thash': '5d816a0',
        'kg-rec': '1',
        'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
    },
};

const req = https.request(options, (res) => {
    let data = '';
    res.on('data', chunk => data += chunk);
    res.on('end', () => {
        console.log('STATUS:', res.statusCode);
        console.log('HEADERS:');
        Object.entries(res.headers).forEach(([k, v]) => console.log(`  ${k}: ${v}`));
        console.log('BODY:', data);
    });
});
req.on('error', e => console.error('ERROR:', e));
req.end();
