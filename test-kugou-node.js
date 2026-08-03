// test-kugou.js - 用 Node.js 测试酷狗 API
const https = require('https');
const http = require('http');
const crypto = require('crypto');

const APPID = "1005";
const CLIENTVER = "20489";
const SALT_WEB = "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt";
const SALT_ANDROID = "OIlwieks28dk2k092lksi2UIkp";
const DEFAULT_DFID = "2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";

function md5(s) { return crypto.createHash('md5').update(s).digest('hex'); }

function calculateMid(guid) {
    const digest = md5(guid);
    let val = BigInt(0);
    for (const c of digest) {
        val = val * BigInt(16) + BigInt(parseInt(c, 16));
    }
    return val.toString();
}

function signWeb(params) {
    const keys = Object.keys(params).sort();
    const pairs = keys.map(k => `${k}=${params[k]}`).join('');
    return md5(`${SALT_WEB}${pairs}${SALT_WEB}`);
}

function signAndroid(params, body = "") {
    const keys = Object.keys(params).sort();
    const pairs = keys.map(k => `${k}=${params[k]}`).join('');
    return md5(`${SALT_ANDROID}${pairs}${body}${SALT_ANDROID}`);
}

function httpGet(url, headers) {
    return new Promise((resolve, reject) => {
        const mod = url.startsWith('https') ? https : http;
        const opts = { headers: { 'User-Agent': 'Mozilla/5.0', ...headers } };
        mod.get(url, opts, res => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, data }));
        }).on('error', reject);
    });
}

async function test() {
    const mid = calculateMid(DEFAULT_DFID);
    console.log(`dfid=${DEFAULT_DFID}`);
    console.log(`mid=${mid}`);

    // Step 1: Search
    console.log('\n=== Search ===');
    const ct = Math.floor(Date.now() / 1000).toString();
    const sp = { keyword: '夜空中最亮的星', page: '1', pagesize: '3', platform: 'WebFilter' };
    const sUrl = 'https://songsearch.kugou.com/song_search_v2?' + new URLSearchParams(sp).toString();
    const sr = await httpGet(sUrl, { Referer: 'https://www.kugou.com/' });
    const songs = JSON.parse(sr.data).lists || [];
    console.log(`Found ${songs.length} songs`);
    for (const s of songs.slice(0, 3)) {
        console.log(`  ${s.SongName} | ${s.SingerName} | hash=${s.FileHash} album=${s.AlbumId}`);
    }

    // Step 2: v5/url with BIGINT mid
    console.log('\n=== v5/url (BIGINT mid) ===');
    for (const s of songs.slice(0, 2)) {
        const ct2 = Math.floor(Date.now() / 1000).toString();
        const p = {
            album_id: s.AlbumId || '0', area_code: '1',
            hash: s.FileHash.toLowerCase(), ssa_flag: 'is_fromtrack',
            version: '11430', page_id: '151369488', quality: '128',
            album_audio_id: '0', behavior: 'play', pid: '2',
            cmd: '26', pidversion: '3001', IsFreePart: '0',
            ppage_id: '463467626,350369493,788954147',
            cdnBackup: '1', module: '', clientver: '11430',
            dfid: DEFAULT_DFID, mid: mid, uuid: '-',
            appid: '1005', clienttime: ct2,
        };
        const sig = signAndroid(p);
        p.signature = sig;
        const u = 'https://trackercdn.kugou.com/v5/url?' + new URLSearchParams(p).toString();
        const r = await httpGet(u, {
            'dfid': DEFAULT_DFID, 'clienttime': ct2, 'mid': mid,
            'kg-rc': '1', 'kg-thash': '5d816a0',
            'x-router': 'trackercdn.kugou.com',
            'User-Agent': 'Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36',
        });
        const d = JSON.parse(r.data);
        console.log(`  ${s.SongName}: errcode=${d.errcode} url=${d.url ? 'YES' : 'NO'}`);
        if (d.url) console.log(`    -> ${d.url.substring(0, 100)}`);
    }

    // Step 3: v5/url with HEX mid (like before)
    console.log('\n=== v5/url (HEX mid) ===');
    const midHex = md5(DEFAULT_DFID);
    for (const s of songs.slice(0, 2)) {
        const ct3 = Math.floor(Date.now() / 1000).toString();
        const p2 = {
            album_id: s.AlbumId || '0', area_code: '1',
            hash: s.FileHash.toLowerCase(), ssa_flag: 'is_fromtrack',
            version: '11430', page_id: '151369488', quality: '128',
            album_audio_id: '0', behavior: 'play', pid: '2',
            cmd: '26', pidversion: '3001', IsFreePart: '0',
            ppage_id: '463467626,350369493,788954147',
            cdnBackup: '1', module: '', clientver: '11430',
            dfid: DEFAULT_DFID, mid: midHex, uuid: '-',
            appid: '1005', clienttime: ct3,
        };
        const sig2 = signAndroid(p2);
        p2.signature = sig2;
        const u2 = 'https://trackercdn.kugou.com/v5/url?' + new URLSearchParams(p2).toString();
        const r2 = await httpGet(u2, {
            'dfid': DEFAULT_DFID, 'clienttime': ct3, 'mid': midHex,
            'kg-rc': '1', 'kg-thash': '5d816a0',
            'x-router': 'trackercdn.kugou.com',
            'User-Agent': 'Mozilla/5.0 (Linux; Android 13; 2304FPN6DC) AppleWebKit/537.36',
        });
        const d2 = JSON.parse(r2.data);
        console.log(`  ${s.SongName}: errcode=${d2.errcode} url=${d2.url ? 'YES' : 'NO'}`);
        if (d2.url) console.log(`    -> ${d2.url.substring(0, 100)}`);
    }

    // Step 4: v1 API (getdata)
    console.log('\n=== v1 getdata ===');
    for (const s of songs.slice(0, 2)) {
        const ct4 = Math.floor(Date.now() / 1000).toString();
        const p3 = {
            r: 'play/getdata', hash: s.FileHash.toLowerCase(),
            album_id: s.AlbumId || '0',
            dfid: DEFAULT_DFID, mid: mid, clientver: '11309',
            appid: '1005', clienttime: ct4,
        };
        const sig3 = signWeb(p3);
        p3.signature = sig3;
        const u3 = 'https://www.kugou.com/yy/index.php?' + new URLSearchParams(p3).toString();
        const r3 = await httpGet(u3, {
            'Referer': 'https://www.kugou.com/',
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
            'Cookie': `dfid=${DEFAULT_DFID}`,
        });
        try {
            const d3 = JSON.parse(r3.data);
            const d = d3.data || {};
            console.log(`  ${s.SongName}: err_code=${d3.err_code} url=${d.play_url ? 'YES' : 'NO'}`);
            if (d.play_url) console.log(`    -> ${d.play_url.substring(0, 100)}`);
        } catch(e) {
            console.log(`  ${s.SongName}: parse error, ${r3.data.substring(0, 200)}`);
        }
    }
}

test().catch(console.error);
