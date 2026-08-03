// test-v6-api.cjs — 测试酷狗 v6 播放 API（详细调试版）
const crypto = require('crypto');
const http = require('http');

const LITE_APPID = '3116';
const LITE_CLIENTVER = '11440';
const SIGN_SALT = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
const SIGNKEY_SALT = '185672dd44712f60bb1736df5a377e82';

function md5Hex(s) { return crypto.createHash('md5').update(s).digest('hex'); }

function trackerKey(hash, mid, userid) {
    return md5Hex(hash + SIGNKEY_SALT + LITE_APPID + mid + userid);
}

function androidSign(params, bodyJson) {
    const sorted = params.slice().sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
    const kv = sorted.map(([k, v]) => k + v).join('');
    return md5Hex(SIGN_SALT + kv + bodyJson + SIGN_SALT);
}

function doRequest(label, hash, dfid, mid, clienttime, includeSig, includeClienttimeUrl) {
    const body = {
        area_code: '1', behavior: 'play',
        qualities: ['128', '320', 'flac', 'high', 'multitrack'],
        resource: {
            album_audio_id: 0, collect_list_id: '3', collect_time: 0,
            hash: hash, id: 0, page_id: 1, type: 'audio'
        },
        token: '', vip: 0, userid: '0',
        tracker_param: {
            all_m: 1, auth: '', is_free_part: 0,
            key: trackerKey(hash, mid, 0),
            module_id: 0, need_climax: 1, need_xcdn: 1,
            open_time: '', pid: '411', pidversion: '3001',
            priv_vip_type: '6', viptoken: ''
        }
    };
    const bodyJson = JSON.stringify(body);

    // 签名参数
    const sigParams = [
        ['appid', LITE_APPID],
        ['clientver', LITE_CLIENTVER],
        ['dfid', dfid],
        ['mid', mid]
    ];
    if (includeClienttimeUrl) sigParams.push(['clienttime', String(clienttime)]);

    const sig = androidSign(sigParams, bodyJson);

    let path = '/v6/priv_url';
    if (includeSig) {
        const q = new URLSearchParams();
        q.append('appid', LITE_APPID);
        q.append('clientver', LITE_CLIENTVER);
        q.append('dfid', dfid);
        q.append('mid', mid);
        q.append('sig', sig);
        if (includeClienttimeUrl) q.append('clienttime', String(clienttime));
        path = '/v6/priv_url?' + q.toString();
    }

    console.log(`\n=== ${label} ===`);
    console.log('path:', path.slice(0, 120) + '...');

    const headers = {
        'User-Agent': 'Android15-1070-11083-46-0-DiscoveryDRADProtocol-wifi',
        'x-router': 'tracker.kugou.com',
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(bodyJson)
    };

    return new Promise((resolve) => {
        const req = http.request({ hostname: 'tracker.kugou.com', port: 80, path, method: 'POST', headers }, (res) => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => {
                try {
                    const json = JSON.parse(data);
                    if (json.error_code === 0 && json.url) {
                        console.log('✅ error_code=0, URL len:', json.url.length, '| quality:', json.quality);
                    } else {
                        console.log('❌ error_code:', json.error_code, '| msg:', json.message || json.msg);
                    }
                } catch(e) { console.log('Raw:', data.slice(0, 200)); }
                resolve();
            });
        });
        req.on('error', e => { console.error('Error:', e.message); resolve(); });
        req.write(bodyJson);
        req.end();
    });
}

async function main() {
    const hash = 'B3A52A7A958BF0AED0EBFBA2E9A818B7'.toLowerCase();
    const dfid = '2a1b2c3d4e5f6a7b8c9d0e1f';
    const mid = '102278954484407344745969333521447052680';
    const clienttime = Math.floor(Date.now() / 1000);

    // 测试1: 无签名，无URL参数（baseline）
    await doRequest('无签名无URL', hash, dfid, mid, clienttime, false, false);
    await delay(500);

    // 测试2: 有签名，无clienttime in URL
    await doRequest('有签名(不含clienttime)', hash, dfid, mid, clienttime, true, false);
    await delay(500);

    // 测试3: 有签名，有clienttime in URL
    await doRequest('有签名(含clienttime)', hash, dfid, mid, clienttime, true, true);
}

function delay(ms) { return new Promise(r => setTimeout(r, ms)); }
main().then(() => console.log('\nDone'));
