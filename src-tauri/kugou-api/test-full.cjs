// 一脚本全部跑完：生成QR → 等待扫码 → 歌单 → 歌曲 → 播放
process.env.platform = 'lite';
const api = require('./main');
const fs = require('fs');

(async () => {
  // Step 1: 生成二维码
  const qr = await api.login_qr_key();
  const key = qr.body.data.qrcode;
  const b64 = qr.body.data.qrcode_img.split(',')[1];
  fs.writeFileSync(__dirname + '/qr.png', Buffer.from(b64, 'base64'));
  console.log('=== 请用酷狗APP扫 qr.png ===');
  console.log('key:', key);

  // Step 2: 轮询扫码 (每2秒，等40次)
  let token, userid;
  for (let i = 0; i < 40; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const ck = await api.login_qr_check({ key, cookie: {} });
    const s = ck.body?.data?.status;
    console.log(`[${i+1}] status=${s}`);
    if (s === 4) {
      token = ck.body.data.token;
      userid = ck.body.data.userid;
      break;
    }
    if (s === 0) { console.log('过期！'); process.exit(1); }
  }
  if (!token) { console.log('超时'); process.exit(1); }
  console.log('✅ 登录成功 userid=' + userid);
  const cookie = { token, userid };

  // Step 3: 歌单 (data.info!)
  const pl = await api.user_playlist({ cookie });
  const list = pl.body.data.info || pl.body.data.data || [];
  console.log('\n=== 歌单 (' + list.length + '个) ===');
  list.forEach((p,i) => console.log(`[${i}] ${p.name||'?'} lid=${p.list_create_listid} mcount=${p.m_count}`));
  if (!list.length) { console.log('无歌单'); process.exit(1); }

  // Step 4: 歌曲 (用 list_create_listid)
  const like = list.find(p => p.m_count > 0) || list[0];
  console.log(`\n=== 歌曲("${like.name}") ===`);
  const ts = await api.playlist_track_all_new({ cookie, listid: like.list_create_listid, pagesize: 5, page: 1 });
  console.log('err:', ts.body?.error_code);
  const sl = ts.body?.data?.data || ts.body?.data?.info || [];
  console.log('歌曲数:', sl.length);
  sl.slice(0,3).forEach((s,i) => console.log(`[${i}] ${s.name||s.songname} hash=${s.hash}`));
  if (!sl.length) { process.exit(1); }

  // Step 5: 播放
  console.log(`\n=== 播放(hash=${sl[0].hash}) ===`);
  const play = await api.song_url({ cookie, hash: sl[0].hash, album_id: sl[0].album_id||0, album_audio_id: sl[0].album_audio_id||0 });
  console.log('err:', play.body?.error_code);
  const url = play.body?.data?.url || play.body?.url;
  console.log(url ? '✅ '+url.substring(0,120) : '⚠️ EMPTY body:'+JSON.stringify(play.body).substring(0,300));
})();
