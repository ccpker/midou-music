// test-api.cjs — 通过本地 KuGouMusicApi 代理请求 v6
// 1. npm install kugou-music-api@latest (or clone from GitHub)
// 2. node test-api.cjs

const http = require('http');

const HASH = 'B3A52A7A958BF0AED0EBFBA2E9A818B7';

async function request(method, host, path, headers, body) {
    return new Promise((resolve) => {
        const opts = { hostname: host, port: 3000, path, method, headers };
        const req = http.request(opts, (res) => {
            let data = '';
            res.on('data', c => data += c);
            res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body: data }));
        });
        req.on('error', e => resolve({ error: e.message }));
        if (body) req.write(body);
        req.end();
    });
}

async function main() {
    // 1. 先测试 song_url_v6（如果没有，返回 song_url_new）
    // 注意：path = /song_url_new?hash=...
    
    // 测试 GET 参数方式
    console.log('\n--- GET /song_url_new?hash=' + HASH + ' ---');
    const r1 = await request('GET', 'localhost', `/song_url_new?hash=${HASH}`, {
        'User-Agent': 'Mozilla/5.0'
    });
    console.log('Status:', r1.status);
    if (r1.body) {
        try {
            const j = JSON.parse(r1.body);
            console.log('error_code:', j.error_code);
            console.log('msg:', j.message || j.msg);
            if (j.url) console.log('url:', j.url.slice(0, 80) + '...');
            if (j.data && j.data[0]) console.log('data[0].url:', j.data[0].url ? j.data[0].url.slice(0,80)+'...' : 'null');
        } catch(e) {
            console.log('body:', r1.body.slice(0, 300));
        }
    }
    if (r1.error) console.log('error:', r1.error);
    
    // 2. 测试 POST /song_url_new
    console.log('\n--- POST /song_url_new body={hash} ---');
    const body = JSON.stringify({ hash: HASH });
    const r2 = await request('POST', 'localhost', '/song_url_new', {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(body)
    }, body);
    console.log('Status:', r2.status);
    if (r2.body) {
        try {
            const j = JSON.parse(r2.body);
            console.log('error_code:', j.error_code);
            console.log('msg:', j.message || j.msg);
            if (j.url) console.log('url:', j.url.slice(0, 80) + '...');
        } catch(e) {
            console.log('body:', r2.body.slice(0, 300));
        }
    }
}

main().catch(console.error);
