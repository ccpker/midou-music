// test-kmr-real.cjs — 搜索真实歌曲 → kmr v1 API 测试
const crypto = require('crypto');
const http = require('http');

const UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36';

function httpGet(host, port, path, headers) {
    return new Promise((resolve, reject) => {
        const req = http.request({ hostname: host, port: port || 80, path, method: 'GET', headers }, res => {
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
        const req = http.request({ hostname: host, port: port || 80, path, method: 'POST', headers: h }, res => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => resolve({ status: res.statusCode, body: data }));
        });
        req.on('error', reject);
        req.write(bodyJson);
        req.end();
    });
}

async function testKmr(hash, label) {
    const liteAppid = '3116', liteClientver = '11440';
    const liteSalt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
    const tsm = Date.now();
    const key = crypto.createHash('md5').update(`${liteAppid}${liteSalt}${liteClientver}${tsm}`).digest('hex');

    const kmrBody = {
        appid: liteAppid,
        clienttime: tsm,
        clientver: liteClientver,
        data: [{ hash, audio_id: 0 }],
        dfid: '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6',
        mid: 'null',
        key
    };
    const kr = await httpPost('kmr.service.kugou.com', 80, '/v1/audio/audio', kmrBody, { 'User-Agent': UA, 'x-router': 'kmr.service.kugou.com' });
    try {
        const kj = JSON.parse(kr.body);
        const d = kj.data && kj.data[0];
        const hasUrl = d && d.url && d.url.length > 10;
        console.log(`  ${label} errcode=${kj.errcode} bitrate=${d?.bitrate||'?'} dur=${d?.duration||'?'} url=${hasUrl?'✅':'❌'}`);
        if (hasUrl) console.log(`    → ${d.url.slice(0,100)}`);
        return hasUrl ? d.url : null;
    } catch(e) { console.log(`  ${label}: 解析失败`); return null; }
}

async function main() {
    // 搜普通流行歌（不加 VIP 标签）
    const keyword = encodeURIComponent('夜空中最亮的星');
    const searchUrl = `/song_search_v2?keyword=${keyword}&platform=WebFilter&format=json&page=1&pagesize=5&userid=-1&clientver=&tag=em&filter=2&iscorrection=1&privilege_filter=0`;
    const sr = await httpGet('songsearch.kugou.com', 80, searchUrl, { 'User-Agent': UA, 'Referer': 'https://www.kugou.com/' });
    console.log('Search status:', sr.status);
    const sl = JSON.parse(sr.body);
    const items = sl?.data?.lists || [];
    if (!items.length) { console.log('No results'); return; }

    // 提取前5首歌的hash，过滤VIP保护
    const candidates = items.filter(s => s.Duration < 300).slice(0, 5);
    console.log(`\n歌曲列表（${candidates.length}首，<300s）：`);
    candidates.forEach((s, i) => {
        console.log(`  ${i+1}. ${s.FileName} [${s.Duration}s] hash=${s.FileHash.slice(0,16)}...`);
    });
    
    let found = null;
    for (const s of candidates) {
        const hash = s.FileHash.toLowerCase().replace(/\s/g, '');
        console.log(`\n测试: ${s.FileName}`);
        const url = await testKmr(hash, 'kmr');
        if (url) { found = url; break; }
        await new Promise(r => setTimeout(r, 400));
    }
    
    if (!found) {
        console.log('\n\n全部失败，尝试 v6/priv_url...');
        // 试 v6
        const hash = candidates[0].FileHash.toLowerCase().replace(/\s/g, '');
        const v6Body = {
            area_code: '1',
            behavior: 'play',
            qualities: ['128', '320', 'flac', 'high', 'multitrack', 'viper_atmos', 'viper_tape', 'viper_clear', 'super'],
            resource: { album_audio_id: 0, collect_list_id: '3', collect_time: Date.now(), hash, id: 0, page_id: 1, type: 'audio' },
            token: '', userid: '0', vip: 0,
            tracker_param: {
                all_m: 1, auth: '', is_free_part: 0, key: '00000000000000000000000000000000',
                module_id: 0, need_climax: 1, need_xcdn: 1, open_time: '', pid: '411',
                pidversion: '3001', priv_vip_type: '6', viptoken: ''
            }
        };
        const liteSalt = 'LnT6xpN3khm36zse0QzvmgTZ3waWdRSA';
        const liteAppid = '3116', liteClientver = '11440';
        const sigParams = [
            ['appid', liteAppid], ['clientver', liteClientver],
            ['dfid', '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6'],
            ['mid', 'null']
        ].map(([k,v]) => `${k}=${v}`).sort().join('');
        const bodyJson = JSON.stringify(v6Body);
        const sig = crypto.createHash('md5').update(`${liteSalt}${sigParams}${bodyJson}${liteSalt}`).digest('hex');
        
        const vr = await httpPost('tracker.kugou.com', 80, '/v6/priv_url', v6Body, {
            'User-Agent': UA,
            'dfid': '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6',
            'mid': 'null',
            'clienttime': Math.floor(Date.now()/1000),
            'kg-rc': '1', 'kg-thash': '5d816a0', 'kg-rec': '1', 'kg-rf': 'B9EDA08A64250DEFFBCADDEE00F8F25F',
            'x-router': 'tracker.kugou.com',
            'signature': sig
        });
        try {
            const vj = JSON.parse(vr.body);
            console.log('v6 errcode:', vj.error_code, '| msg:', vj.message || vj.msg);
            if (vj.url) console.log('✅ v6 URL:', vj.url.slice(0,100));
        } catch(e) { console.log('v6 Raw:', vr.body.slice(0,300)); }
    }
}

main().catch(console.error);
