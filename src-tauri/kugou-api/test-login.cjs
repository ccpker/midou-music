// 酷狗 lite 全链路验证 — 扫码后
process.env.platform = 'lite';
const api = require('./main');

const QR_KEY = '4cabb74ff4860f38c84fa8d9ee2853c01001';

(async () => {
  // ========== 1. 检查扫码状态 ==========
  console.log('=== Step 1: Check QR status ===');
  const check = await api.login_qr_check({ key: QR_KEY, cookie: {} });
  const status = check.body?.data?.status;
  console.log('status:', status, '(4=成功)');

  if (status !== 4) {
    console.log('ERROR: 扫码未成功，当前状态=' + status);
    console.log(JSON.stringify(check.body, null, 2));
    process.exit(1);
  }

  const token = check.body.data.token;
  const userid = check.body.data.userid;
  console.log('登录成功！userid=' + userid + ' token=' + token?.substring(0, 20) + '...');

  const cookie = { token, userid };

  // ========== 2. 歌单列表 ==========
  console.log('\n=== Step 2: Get user playlists ===');
  const playlists = await api.user_playlist({ cookie });
  console.log('status:', playlists.status, 'error_code:', playlists.body?.error_code);
  const plList = playlists.body?.data?.data || playlists.body?.data?.info || [];
  console.log('歌单数量:', plList.length);
  plList.forEach((pl, i) => {
    console.log(`  [${i}] "${pl.name || pl.special_name}" id=${pl.id || pl.global_collection_id} count=${pl.count || pl.song_count}`);
  });

  if (plList.length === 0) { console.log('无歌单，退出'); process.exit(1); }

  // ========== 3. 歌单歌曲 ==========
  const firstPl = plList[0];
  const plId = firstPl.id || firstPl.global_collection_id;
  console.log(`\n=== Step 3: Playlist songs (id=${plId}, "${firstPl.name || firstPl.special_name}") ===`);
  
  const songs = await api.playlist_track_all({ cookie, id: plId, pagesize: 100, page: 1 });
  console.log('status:', songs.status, 'error_code:', songs.body?.error_code);
  const songList = songs.body?.data?.data || songs.body?.data?.info || [];
  console.log('歌曲数量:', songList.length);
  songList.slice(0, 5).forEach((s, i) => {
    console.log(`  [${i}] ${s.name || s.songname} - ${s.author_name || s.singername}  hash=${s.hash} album_audio_id=${s.album_audio_id}`);
  });

  if (songList.length === 0) { console.log('无歌曲'); process.exit(1); }

  // ========== 4. 播放 URL ==========
  const firstSong = songList[0];
  console.log(`\n=== Step 4: Play URL (hash=${firstSong.hash}) ===`);
  
  const playRes = await api.song_url({
    cookie,
    hash: firstSong.hash,
    album_id: firstSong.album_id || 0,
    album_audio_id: firstSong.album_audio_id || 0,
  });
  console.log('status:', playRes.status);
  console.log(JSON.stringify(playRes.body, null, 2));

  const url = playRes.body?.data?.url || playRes.body?.url;
  if (url) {
    console.log('\n✅ 全链路通过！', url.substring(0, 80) + '...');
  } else {
    console.log('\n⚠️ 播放URL为空');
    if (playRes.body?.ssaCode) console.log('SSA:', playRes.body.ssaCode);
  }
})();
