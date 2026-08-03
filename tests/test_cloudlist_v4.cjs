// test_cloudlist_v4 — 修复标准版签名: salt 和 sign 顺序

const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }

// ── 标准版 ──
const SALT_STD = 'NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt';
const APPID_STD = '1005';
const CLIENTVER_STD = '11309';

// ── lite ──
const SALT_LITE = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
const APPID_LITE = '3116';
const CLIENTVER_LITE = '11440';

const TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990';
const DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = md5(DFID);

function doReq(name, appid, cliver, salt, extraParams) {
  return new Promise((resolve) => {
    const ct = Math.floor(Date.now() / 1000);
    const params = {
      appid, clienttime: String(ct), clientver: cliver,
      dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID,
      ...extraParams,
    };
    const body = JSON.stringify({ userid: Number(USERID), token: TOKEN, total_ver: 979, type: 2, page: 1, pagesize: 30 });

    // 签名: MD5(salt + sorted_k=v... + body + salt)
    const sorted = Object.keys(params).sort();
    const kv = sorted.map(k => `${k}=${params[k]}`).join('');
    const sig = md5(`${salt}${kv}${body}${salt}`);
    params.signature = sig;

    const qs = Object.entries(params).map(([k,v]) => `${k}=${encodeURIComponent(v)}`).join('&');
    console.log(`\n── ${name} ──`);
    console.log(`appid=${appid} cliver=${cliver}`);
    console.log(`sig=${sig.substring(0,16)}...`);
    console.log(`params:`, Object.keys(params).join(','));

    const opts = {
      hostname: 'gateway.kugou.com',
      path: `/v7/get_all_list?${qs}`,
      method: 'POST',
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
      res.on('data', c => data += c);
      res.on('end', () => {
        try {
          const j = JSON.parse(data);
          if (j.status === 1 && j.data?.info) {
            console.log('✅ 成功!', j.data.info.length, '个歌单');
            j.data.info.forEach(p => console.log(`  listid=${p.listid} name=${p.name} count=${p.count}`));
          } else {
            console.log(`error_code=${j.error_code} error_msg=${j.error_msg} status=${j.status}`);
          }
        } catch { console.log('raw:', data.substring(0, 300)); }
        resolve();
      });
    });
    req.on('error', e => { console.log(`ERR: ${e.message}`); resolve(); });
    req.write(body);
    req.end();
  });
}

async function main() {
  // 1. 标准版 + plat=4
  await doReq('标准版 plat=4', APPID_STD, CLIENTVER_STD, SALT_STD, { plat: '4' });
  // 2. 标准版 + plat=1
  await doReq('标准版 plat=1', APPID_STD, CLIENTVER_STD, SALT_STD, { plat: '1' });
  // 3. lite + plat=4 (已验证 20017)
  await doReq('lite plat=4', APPID_LITE, CLIENTVER_LITE, SALT_LITE, { plat: '4' });
}
main().catch(console.error);
