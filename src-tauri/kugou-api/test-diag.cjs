// 酷狗 lite 全链路 — 最终版
process.env.platform = 'lite';
const api = require('./main');
const TOKEN = '91a79248ec1dd5faef1e';
const UID = 1514557990;
const cookie = { token: TOKEN, userid: UID };

async function go() {
  // 1. 歌单 — 用 info 而不是 data
  const pl = await api.user_playlist({ cookie });
  const list = pl.body.data.info || pl.body.data.data || [];
  console.log('歌单数:', list.length);
  list.forEach((p, i) => {
    console.log(`[${i}] ${p.name||p.special_name||'?'} lid=${p.list_create_listid} gcid=${p.global_collection_id} mcount=${p.m_count}`);
  });
  if (!list.length) { console.log('无歌单'); return; }

  // 2. 拿有歌曲的歌单
  const like = list.find(p => p.m_count > 0) || list[0];
  console.log(`\n>>> 目标: ${like.name||like.special_name} lid=${like.list_create_listid} mcount=${like.m_count}`);

  // 3. track_all_new (用 list_create_listid)
  const songsRes = await api.playlist_track_all_new({ cookie, listid: like.list_create_listid, pagesize: 5, page: 1 });
  console.log('track_all_new err:', songsRes.body?.error_code);
  const songList = songsRes.body?.data?.data || songsRes.body?.data?.info || [];
  console.log('歌曲数:', songList.length);
  songList.slice(0, 3).forEach((s, i) => {
    console.log(`  [${i}] ${s.name||s.songname||'?'} - ${s.author_name||s.singername||'?'}  hash=${s.hash}`);
  });
  if (!songList.length) return;

  // 4. 播放
  const song = songList[0];
  const playRes = await api.song_url({ cookie, hash: song.hash, album_id: song.album_id||0, album_audio_id: song.album_audio_id||0 });
  console.log('\n播放 err:', playRes.body?.error_code, 'status:', playRes.status);
  const url = playRes.body?.data?.url || playRes.body?.url;
  if (url) {
    console.log('✅ 全链路通过！');
    console.log('URL:', url.substring(0, 120));
  } else {
    console.log('⚠️ URL为空');
    console.log('body:', JSON.stringify(playRes.body).substring(0, 300));
  }
}

go().catch(e => console.log('FATAL:', e.body?.error_code || e.message));
