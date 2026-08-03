// 测试: 酷狗歌单 get_all_list — 对照 MoeKoeMusic 实际链路
// 方案A: 直接调 cloudlist.service.kugou.com
// 方案B: 走 gateway + x-router
// 方案C: 调 gateway 不走 x-router

const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }

const SALT = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
const APPID = '3116';
const CLIENTVER = '11440';
const TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990';
const DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = md5(DFID);

function signParams(params, body) {
  const sorted = Object.keys(params).sort();
  const kv = sorted.map(k => `${k}=${params[k]}`).join('');
  return md5(`${SALT}${kv}${body}${SALT}`);
}

function doRequest(hostname, path, method, bodyObj, extraParams, extraHeaders) {
  return new Promise((resolve, reject) => {
    const ct = Math.floor(Date.now() / 1000);
    const params = {
      appid: APPID,
      clienttime: String(ct),
      clientver: CLIENTVER,
      dfid: DFID,
      mid: MID,
      uuid: '-',
      token: TOKEN,
      userid: USERID,
      ...extraParams,
    };
    const bodyStr = JSON.stringify(bodyObj);
    const sig = signParams(params, bodyStr);
    params.signature = sig;

    const qs = Object.entries(params).map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join('&');
    const fullPath = `${path}?${qs}`;

    const opts = {
      hostname,
      path: fullPath,
      method,
      headers: {
        'Content-Type': 'application/json; charset=utf-8',
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'dfid': DFID,
        'mid': MID,
        'clienttime': String(ct),
        'kg-rc': '1',
        'kg-thash': '5d816a0',
        'kg-rec': '1',
        'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
        ...extraHeaders,
      },
    };

    const req = https.request(opts, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        try { resolve(JSON.parse(data)); }
        catch { resolve({ raw: data.substring(0, 500) }); }
      });
    });
    req.on('error', reject);
    if (bodyStr) req.write(bodyStr);
    req.end();
  });
}

async function main() {
  const body = {
    userid: Number(USERID),
    token: TOKEN,
    total_ver: 979,
    type: 2,
    page: 1,
    pagesize: 30,
  };

  // 方案A: 直接调 cloudlist.service.kugou.com
  console.log('──── 方案A: cloudlist.service.kugou.com 直接 ────');
  try {
    const rA = await doRequest('cloudlist.service.kugou.com', '/v7/get_all_list', 'POST', body, { plat: '4' }, {});
    console.log('status:', rA.status, 'error_code:', rA.error_code, 'info_count:', rA.data?.info?.length);
  } catch(e) { console.log('error:', e.message); }

  // 方案B: gateway + x-router (我们现在的实现)
  console.log('\n──── 方案B: gateway + x-router ────');
  try {
    const rB = await doRequest('gateway.kugou.com', '/v7/get_all_list', 'POST', body, { plat: '4' },
      { 'x-router': 'cloudlist.service.kugou.com' });
    console.log('status:', rB.status, 'error_code:', rB.error_code, 'info_count:', rB.data?.info?.length);
    if (rB.error_code !== 0) console.log('error_msg:', rB.error_msg);
  } catch(e) { console.log('error:', e.message); }

  // 方案C: gateway 不走 x-router
  console.log('\n──── 方案C: gateway 不走 x-router ────');
  try {
    const rC = await doRequest('gateway.kugou.com', '/v7/get_all_list', 'POST', body, { plat: '4' }, {});
    console.log('status:', rC.status, 'error_code:', rC.error_code, 'info_count:', rC.data?.info?.length);
    if (rC.error_code !== 0) console.log('error_msg:', rC.error_msg);
  } catch(e) { console.log('error:', e.message); }

  // 方案D: cloudlist 直接 + plat=1
  console.log('\n──── 方案D: cloudlist 直接 + plat=1 ────');
  try {
    const rD = await doRequest('cloudlist.service.kugou.com', '/v7/get_all_list', 'POST', body, { plat: '1' }, {});
    console.log('status:', rD.status, 'error_code:', rD.error_code, 'info_count:', rD.data?.info?.length);
  } catch(e) { console.log('error:', e.message); }
}

main().catch(console.error);
