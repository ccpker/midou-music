// 扫码后一步到位：搜索+播放，跳过歌单
process.env.platform = 'lite';
const api = require('./main');
const QR_KEY = 'a1a04a23f141ca4063c9d943fd8a0fc01001';

(async () => {
  const ck = await api.login_qr_check({ key: QR_KEY, cookie: {} });
  if (ck.body?.data?.status !== 4) { console.log('先扫码! status=' + ck.body?.data?.status); return; }
  const cookie = { token: ck.body.data.token, userid: ck.body.data.userid };
  console.log('✅ 登录 userid=' + ck.body.data.userid);

  // 搜索
  const sr = await api.search({ cookie, keyword: '晴天', page: 1, pagesize: 3 });
  console.log('搜索 err:', sr.body?.error_code);
  const songs = sr.body?.data?.data || [];
  console.log('结果:', songs.length);
  songs.forEach((s, i) => console.log(`[${i}] ${s.songname} - ${s.singername} hash=${s.hash}`));
  if (!songs.length) { console.log('无搜索结果'); return; }

  // 播放
  const play = await api.song_url({ cookie, hash: songs[0].hash, album_id: songs[0].album_id || 0, album_audio_id: songs[0].album_audio_id || 0 });
  console.log('\n播放 err:', play.body?.error_code);
  console.log(play.body?.data?.url ? '✅ ' + play.body.data.url.substring(0, 120) : '⚠️ EMPTY\n' + JSON.stringify(play.body).substring(0, 500));
})().catch(e => console.log('ERR:', e.body?.error_code, e.status));
