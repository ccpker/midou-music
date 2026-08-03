// test-kmr-deep.cjs
const http = require('http');
const https = require('https');
const crypto = require('crypto');

function httpGet(url, headers) {
    return new Promise((resolve, reject) => {
        const u = new URL(url);
        const mod = u.protocol === 'https:' ? https : http;
        const req = mod.request({ hostname: u.hostname, port: u.port||(u.protocol==='https:'?443:80), path: u.pathname+u.search, method:'GET', headers }, res => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => resolve({ status: res.statusCode, body: data }));
        });
        req.on('error', reject);
        req.end();
    });
}

function httpPost(host, port, path, body, headers) {
    const bodyJson = JSON.stringify(body);
    const h = { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(bodyJson), ...headers };
    return new Promise((resolve, reject) => {
        const req = http.request({ hostname: host, port: port||80, path, method: 'POST', headers: h }, res => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => resolve({ status: res.statusCode, body: data }));
        });
        req.on('error', reject);
        req.write(bodyJson);
        req.end();
    });
}

async function testKmr(label, body, extraHeaders) {
    const kr = await httpPost('kmr.service.kugou.com', 80, '/v1/audio/audio', body, {
        'User-Agent': 'Mozilla/5.0', 'x-router': 'kmr.service.kugou.com', ...extraHeaders
    });
    try {
        const kj = JSON.parse(kr.body);
        const d = kj.data && kj.data[0];
        const hasUrl = d && d.url && d.url.length > 10;
        console.log(`  ${label}: errcode=${kj.errcode} url=${hasUrl?'YES':'NO'} bitrate=${d?.bitrate||'?'}`);
        if (hasUrl) console.log(`    -> ${d.url.slice(0,100)}`);
        return hasUrl ? d.url : null;
    } catch(e) { console.log(`  ${label}: parse fail`); return null; }
}

async function main() {
    const liteAppid = '3116', liteClientver = '11440';
    const liteSalt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
    const tsm = Date.now();
    const key = crypto.createHash('md5').update(`${liteAppid}${liteSalt}${liteClientver}${tsm}`).digest('hex');
    const base = { appid: liteAppid, clienttime: tsm, clientver: liteClientver, dfid: '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6', key };

    const kw = encodeURIComponent('夜空中最亮的星');
    const sr = await httpGet(`http://songsearch.kugou.com/song_search_v2?keyword=${kw}&platform=WebFilter&format=json&page=1&pagesize=3&userid=-1&clientver=&tag=em&filter=2&iscorrection=1&privilege_filter=0`, { 'User-Agent': 'Mozilla/5.0', 'Referer': 'https://www.kugou.com/' });
    const sl = JSON.parse(sr.body);
    const items = (sl?.data?.lists || []).slice(0, 3);
    console.log(`Search: ${items.length} results\n`);

    for (const s of items) {
        const hash16 = s.FileHash.toLowerCase().replace(/\s/g, '');
        const hash32 = hash16.padEnd(32, '0');
        console.log(`> ${s.FileName} [${s.Duration}s] hash=${hash16}`);

        await testKmr('mid="null" 16h', { ...base, data: [{ hash: hash16, audio_id: 0 }], mid: 'null' });
        await new Promise(r => setTimeout(r, 300));
        await testKmr('mid="null" 32h', { ...base, data: [{ hash: hash32, audio_id: 0 }], mid: 'null' });
        await new Promise(r => setTimeout(r, 300));
        await testKmr('no mid 16h', { ...base, data: [{ hash: hash16, audio_id: 0 }] });
        await new Promise(r => setTimeout(r, 300));
        await testKmr('no mid 32h', { ...base, data: [{ hash: hash32, audio_id: 0 }] });
        console.log('');
    }
}

main().catch(console.error);
