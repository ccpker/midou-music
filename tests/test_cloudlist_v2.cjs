// test_cloudlist_v2.cjs — 只测方案B vs 不同参数组合

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

async function test(name, hostname, path, method, bodyObj, extraParams, extraHeaders) {
  return new Promise((resolve) => {
    const ct = Math.floor(Date.now() / 1000);
    const params = {
      appid: APPID, clienttime: String(ct), clientver: CLIENTVER,
      dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID,
      ...extraParams,
    };
    const bodyStr = JSON.stringify(bodyObj);
    const sig = signParams(params, bodyStr);
    params.signature = sig;

    const qs = Object.entries(params).map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join('&');
    const fullPath = `${path}?${qs}`;

    const opts = {
      hostname, path: fullPath, method,
      headers: {
        'Content-Type': 'application/json; charset=utf-8',
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'dfid': DFID, 'mid': MID, 'clienttime': String(ct),
        'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
        ...extraHeaders,
      },
    };

    const req = https.request(opts, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        console.log(`\n── ${name} ──`);
        try {
          const j = JSON.parse(data);
          console.log('status:', j.status, 'error_code:', j.error_code, 'error_msg:', j.error_msg);
          console.log('data keys:', j.data ? Object.keys(j.data).slice(0, 10).join(',') : 'N/A');
          if (j.data?.info) console.log('info count:', j.data.info.length);
          console.log('raw(500):', data.substring(0, 500));
        } catch(e) {
          console.log('raw:', data.substring(0, 800));
        }
        resolve();
      });
    });
    req.on('error', (e) => { console.log(`${name}: ERR ${e.message}`); resolve(); });
    if (bodyStr) req.write(bodyStr);
    req.end();
  });
}

async function main() {
  const body = { userid: Number(USERID), token: TOKEN, total_ver: 979, type: 2, page: 1, pagesize: 30 };

  // 1: gateway + x-router, plat=4 (当前)
  await test('gateway+x-router plat=4', 'gateway.kugou.com', '/v7/get_all_list', 'POST', body, { plat: '4' },
    { 'x-router': 'cloudlist.service.kugou.com' });

  // 2: gateway + x-router, 不加 plat
  await test('gateway+x-router 无plat', 'gateway.kugou.com', '/v7/get_all_list', 'POST', body, {},
    { 'x-router': 'cloudlist.service.kugou.com' });

  // 3: gateway + x-router, plat=2 (PC)
  await test('gateway+x-router plat=2', 'gateway.kugou.com', '/v7/get_all_list', 'POST', body, { plat: '2' },
    { 'x-router': 'cloudlist.service.kugou.com' });

  // 4: gateway + x-router, plat=0
  await test('gateway+x-router plat=0', 'gateway.kugou.com', '/v7/get_all_list', 'POST', body, { plat: '0' },
    { 'x-router': 'cloudlist.service.kugou.com' });

  // 5: 去掉 total_ver
  const body2 = { userid: Number(USERID), token: TOKEN, type: 2, page: 1, pagesize: 30 };
  await test('gateway+x-router 无total_ver', 'gateway.kugou.com', '/v7/get_all_list', 'POST', body2, { plat: '4' },
    { 'x-router': 'cloudlist.service.kugou.com' });

  // 6: type=0
  const body3 = { userid: Number(USERID), token: TOKEN, total_ver: 979, type: 0, page: 1, pagesize: 30 };
  await test('gateway+x-router type=0', 'gateway.kugou.com', '/v7/get_all_list', 'POST', body3, { plat: '4' },
    { 'x-router': 'cloudlist.service.kugou.com' });

  // 7: 不带 token (看看是不是 token 过期了)
  const body4 = { userid: Number(USERID), type: 2, page: 1, pagesize: 30 };
  const extra7 = { plat: '4' };
  // 手动去掉token from extra params
  await new Promise((resolve) => {
    const ct = Math.floor(Date.now() / 1000);
    const params = {
      appid: APPID, clienttime: String(ct), clientver: CLIENTVER,
      dfid: DFID, mid: MID, uuid: '-', userid: USERID, plat: '4',
    };
    const bodyStr = JSON.stringify(body4);
    const sig = signParams(params, bodyStr);
    params.signature = sig;
    const qs = Object.entries(params).map(([k, v]) => `${k}=${encodeURIComponent(v)}`).join('&');
    const opts = {
      hostname: 'gateway.kugou.com', path: `/v7/get_all_list?${qs}`, method: 'POST',
      headers: {
        'Content-Type': 'application/json; charset=utf-8',
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'dfid': DFID, 'mid': MID, 'clienttime': String(ct),
        'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
        'x-router': 'cloudlist.service.kugou.com',
      },
    };
    const req = https.request(opts, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        console.log('\n── 无token ──');
        console.log('raw(500):', data.substring(0, 500));
        resolve();
      });
    });
    req.on('error', (e) => { console.log('无token ERR:', e.message); resolve(); });
    req.write(bodyStr);
    req.end();
  });
}

main().catch(console.error);
