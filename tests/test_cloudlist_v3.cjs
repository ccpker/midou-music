// test_cloudlist_v3 — 用标准版 appid=1005 测 cloudlist

const crypto = require('crypto');
const https = require('https');

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }

// 标准版常量
const SALT_STD = 'NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt';
const APPID_STD = '1005';
const CLIENTVER_STD = '11309';
const TOKEN = '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = '1514557990';
const DFID = '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = md5(DFID);

function sigStd(params, body) {
  const sorted = Object.keys(params).sort();
  const kv = sorted.map(k => `${k}=${params[k]}`).join('');
  return md5(`${SALT_STD}${kv}${body}${SALT_STD}`);
}

function doReq(name, hostname, path, bodyObj, extraParams) {
  return new Promise((resolve) => {
    const ct = Math.floor(Date.now() / 1000);
    const params = {
      appid: APPID_STD, clienttime: String(ct), clientver: CLIENTVER_STD,
      dfid: DFID, mid: MID, uuid: '-', token: TOKEN, userid: USERID,
      ...extraParams,
    };
    const bodyStr = JSON.stringify(bodyObj);
    const sig = sigStd(params, bodyStr);
    params.signature = sig;
    const qs = Object.entries(params).map(([k,v]) => `${k}=${encodeURIComponent(v)}`).join('&');

    const opts = {
      hostname, path: `${path}?${qs}`, method: 'POST',
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
        console.log(`\n── ${name} ──`);
        try {
          const j = JSON.parse(data);
          if (j.status === 1 && j.data?.info) {
            console.log('✅ 成功!', j.data.info.length, '个歌单');
            j.data.info.forEach(p => console.log(`  listid=${p.listid} name=${p.name} count=${p.count}`));
          } else {
            console.log('error_code:', j.error_code, 'status:', j.status);
          }
        } catch { console.log('raw:', data.substring(0, 300)); }
        resolve();
      });
    });
    req.on('error', e => { console.log(`${name}: ERR ${e.message}`); resolve(); });
    req.write(bodyStr);
    req.end();
  });
}

async function main() {
  const body = { userid: Number(USERID), token: TOKEN, total_ver: 979, type: 2, page: 1, pagesize: 30 };

  // 标准版 appid=1005
  await doReq('标准版 plat=4', 'gateway.kugou.com', '/v7/get_all_list', body, { plat: '4' });
  await doReq('标准版 plat=2', 'gateway.kugou.com', '/v7/get_all_list', body, { plat: '2' });
  await doReq('标准版 plat=1', 'gateway.kugou.com', '/v7/get_all_list', body, { plat: '1' });
}

main().catch(console.error);
