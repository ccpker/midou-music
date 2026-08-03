// 最小单元验证：MoeKoeMusic 原版 v5 URL，lite 模式
// 用法: set KUGOU_API_MID=xxx && node test_song_url.cjs

const path = require('path');
const apiDir = path.resolve('D:/workspaces/search/projects/_research/MoeKoeMusic/api');

// 设置 platform=lite
process.env.platform = 'lite';

// 从我们的 app 拿 token/mid/dfid
const TOKEN = process.env.KUGOU_TOKEN || '91a79248ec1dd5faef1eb8e13b364591776510ff0bb954ca623698a79f5ab239';
const USERID = process.env.KUGOU_USERID || '1514557990';
const DFID = process.env.KUGOU_DFID || '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6';
const MID = process.env.KUGOU_API_MID || '4b0a5cb94103098b612eecd6f9d4cc08';

const testHashes = [
  '7db52b1c2250e182627836d740ebbb04',  // 王琪 情人迷  (歌单 我喜欢)
  '3970e49f52b3097d1b477cc35ed7da46',  // 另一首
];

async function main() {
  const { createRequest } = require(path.join(apiDir, 'util/request.js'));

  for (const hash of testHashes) {
    console.log(`\n=== 测试 hash=${hash} ===`);
    try {
      const result = await createRequest({
        url: '/v5/url',
        method: 'GET',
        baseURL: 'https://gateway.kugou.com',
        params: {
          album_id: 0,
          area_code: 1,
          hash: hash,
          ssa_flag: 'is_fromtrack',
          version: 11430,
          page_id: 967177915,       // lite
          quality: 128,
          album_audio_id: 0,
          behavior: 'play',
          pid: 411,                 // lite
          cmd: 26,
          pidversion: 3001,
          IsFreePart: 0,
          ppage_id: '356753938,823673182,967485191', // lite
          cdnBackup: 1,
          module: '',
          clientver: 11430,
        },
        encryptType: 'android',
        headers: { 'x-router': 'trackercdn.kugou.com' },
        encryptKey: true,
        cookie: {
          dfid: DFID,
          KUGOU_API_MID: MID,
          token: TOKEN,
          userid: USERID,
        },
      });

      console.log('✅ 成功!');
      const body = result.body;
      console.log('status:', body.status);
      console.log('url:', body.url || 'null');
      console.log('errcode:', body.errcode);
      console.log('error:', body.error || 'none');
      console.log('bitrate:', body.bitrate);
      console.log(JSON.stringify(body, null, 2).slice(0, 500));
    } catch (err) {
      console.log('❌ 失败: status=', err.status);
      const b = err.body || {};
      console.log('errcode:', b.errcode);
      console.log('error:', b.error || b.errmsg || '?');
      console.log('ssaCode:', b.ssaCode || 'none');
      console.log('edt:', b.edt || 'none');
      console.log('sid:', b.sid || 'none');
      console.log(JSON.stringify(b, null, 2).slice(0, 500));
    }
  }
}

main().catch(e => console.error('FATAL:', e));
