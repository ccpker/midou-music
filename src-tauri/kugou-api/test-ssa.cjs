// 测试 SSA 全链路
const http = require('http');

function get(path, qs) {
  return new Promise((resolve, reject) => {
    http.get(`http://localhost:3000${path}?${qs}`, res => {
      let d = ''; res.on('data', c => d += c); res.on('end', () => {
        try { resolve(JSON.parse(d)); } catch(e) { reject(e); }
      });
    }).on('error', reject);
  });
}

(async () => {
  // 1. 登录 (用之前拿到的 token)
  // 先重新扫码拿 fresh token
  const qr = await get('/login/qr/key', 'timestamp=' + Date.now());
  const key = qr.data.qrcode;
  console.log('请扫码 key=' + key);

  let token, userid;
  for (let i = 0; i < 60; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const ck = await get('/login/qr/check', 'key=' + key + '&timestamp=' + Date.now());
    if (ck.data?.status === 4) {
      token = ck.data.token; userid = ck.data.userid;
      console.log('✅ 登录 userid=' + userid);
      break;
    }
    if (ck.data?.status === 0) process.exit(1);
  }
  if (!token) process.exit(1);

  const auth = 'userid=' + userid + '&token=' + token;

  // 2. 搜索+播放 → 触发 20028
  const sr = await get('/search', auth + '&keyword=晴天&pagesize=1&timestamp=' + Date.now());
  const song = (sr.data?.data || [])[0];
  if (!song) { console.log('无搜索结果'); process.exit(1); }
  console.log('搜索:', song.songname, song.hash);

  const play = await get('/song/url', auth + '&hash=' + song.hash + '&album_id=' + (song.album_id||0) + '&album_audio_id=' + (song.album_audio_id||0) + '&timestamp=' + Date.now());
  console.log('\n播放响应:');
  console.log(JSON.stringify(play, null, 2));

  if (play.errcode === 20028) {
    const eventid = play.edt;
    console.log('\n=== SSA: eventid=' + eventid.substring(0, 30) + '... ===');

    // 3. get_verify_info
    const vi = await get('/get/verify/info', auth + '&eventid=' + encodeURIComponent(eventid) + '&timestamp=' + Date.now());
    console.log('\nget_verify_info:');
    console.log(JSON.stringify(vi, null, 2));
  }
})().catch(e => console.log('ERR:', e.message));
