// 酷狗 lite 全链路 — 用"我喜欢" + 播放
process.env.platform = 'lite';
const api = require('./main');

const TOKEN = '91a79248ec1dd5faef1e';
const USERID = 1514557990;
const cookie = { token: TOKEN, userid: USERID };

(async () => {
  // ========== 1. 歌单歌曲 — "我喜欢" ==========
  const plId = 'collection_3_1514557990_2_0';
  console.log('=== Get playlist songs (我喜欢, id=' + plId + ') ===');
  
  // 先试 playlist_track_all
  let songs = await api.playlist_track_all({ cookie, id: plId, pagesize: 100, page: 1 });
  console.log('track_all:', songs.status, 'err:', songs.body?.error_code);
  
  let songList = songs.body?.data?.data || songs.body?.data?.info || [];
  console.log('歌曲数:', songList.length);

  if (songList.length === 0) {
    // 试 track_all_new
    console.log('trying track_all_new...');
    songs = await api.playlist_track_all_new({ cookie, id: plId, pagesize: 100, page: 1 });
    console.log('track_all_new:', songs.status, 'err:', songs.body?.error_code);
    songList = songs.body?.data?.data || songs.body?.data?.info || [];
    console.log('歌曲数:', songList.length);
  }

  if (songList.length === 0) { console.log('无歌曲'); process.exit(1); }

  songList.slice(0, 5).forEach((s, i) => {
    console.log(`  [${i}] ${s.name || s.songname} - ${s.author_name || s.singername}  hash=${s.hash} aaid=${s.album_audio_id}`);
  });

  // ========== 2. 播放 URL ==========
  const firstSong = songList[0];
  console.log(`\n=== Play URL (${firstSong.name || firstSong.songname}, hash=${firstSong.hash}) ===`);
  
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
    console.log('\n✅ 全链路通过！');
    console.log('URL:', url.substring(0, 120));
    console.log('bitrate:', playRes.body?.data?.bitrate || playRes.body?.bitrate);
  } else {
    console.log('\n⚠️ URL为空');
  }
})();
