// 用刚扫的码验证全链路
process.env.platform = 'lite';
const api = require('./main');

const QR_KEY = '5b0124a868c076b78213477971d666501001';

(async () => {
  // 1. 检查扫码
  console.log('=== 检查扫码 ===');
  let token, userid;
  for (let i = 0; i < 5; i++) {
    const check = await api.login_qr_check({ key: QR_KEY, cookie: {} });
    const status = check.body?.data?.status;
    console.log(`[${i+1}] status=${status}`);
    if (status === 4) {
      token = check.body.data.token;
      userid = check.body.data.userid;
      break;
    }
    await new Promise(r => setTimeout(r, 1000));
  }
  if (!token) { console.log('扫码未成功'); process.exit(1); }
  console.log('userid=' + userid + ' token=' + token.substring(0, 16) + '...');

  const cookie = { token, userid };

  // 2. 歌单
  console.log('\n=== 歌单 ===');
  const pl = await api.user_playlist({ cookie });
  console.log('err:', pl.body?.error_code);
  const list = pl.body?.data?.data || [];
  console.log('count:', list.length);
  list.forEach((p,i) => console.log(`[${i}] ${p.name||p.special_name} id=${p.id} cid=${p.global_collection_id} count=${p.count||p.song_count}`));
  if (!list.length) process.exit(1);

  // 3. 歌曲 — "我喜欢"
  const like = list.find(p => (p.name||p.special_name)==='我喜欢') || list[0];
  console.log(`\n=== 歌曲(${like.name}) [try1: track_all, try2: track_all_new] ===`);
  let songs = await api.playlist_track_all({ cookie, id: like.global_collection_id||like.id, pagesize: 100 });
  let songList = songs.body?.data?.data || [];
  console.log('track_all:', songList.length, 'err:', songs.body?.error_code);
  
  if (!songList.length) {
    try {
      songs = await api.playlist_track_all_new({ cookie, listid: like.id, pagesize: 100 });
      songList = songs.body?.data?.data || [];
      console.log('track_all_new:', songList.length, 'err:', songs.body?.error_code);
    } catch(e) { console.log('track_all_new FAIL:', e.body?.error_code); }
  }
  songList.slice(0,3).forEach((s,i) => console.log(`[${i}] ${s.name||s.songname} hash=${s.hash}`));
  if (!songList.length) process.exit(1);

  // 4. 播放
  const s = songList[0];
  console.log(`\n=== 播放(hash=${s.hash}) ===`);
  const play = await api.song_url({ cookie, hash: s.hash, album_id: s.album_id||0, album_audio_id: s.album_audio_id||0 });
  console.log('err:', play.body?.error_code, 'status:', play.status);
  const url = play.body?.data?.url || play.body?.url;
  console.log(url ? '✅ URL: '+url.substring(0,100) : '⚠️ EMPTY. body:'+JSON.stringify(play.body).substring(0,300));
})();
