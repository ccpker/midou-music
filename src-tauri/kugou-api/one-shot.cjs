// 绕过搜索，直接测 歌单→歌单歌曲→播放
const http = require('http');
const fs = require('fs');
const { execSync } = require('child_process');

function api(p, q) {
  return new Promise((resolve, reject) => {
    http.get(`http://localhost:3000${p}?${q}`, res => {
      let d = ''; res.on('data', c => d += c); res.on('end', () => { try { resolve(JSON.parse(d)); } catch(e) { reject(e); } });
    }).on('error', reject);
  });
}

(async () => {
  const qr = await api('/login/qr/key', 't=' + Date.now());
  const key = qr.data.qrcode;
  const imgB64 = qr.data.qrcode_img.split(',')[1];
  fs.writeFileSync(__dirname + '/qr.png', Buffer.from(imgB64, 'base64'));
  execSync('start "" "' + __dirname.replace(/\\/g,'\\\\') + '\\\\qr.png"');
  console.log('key=' + key);
  console.log('请扫码!');

  let token, userid;
  for (let i = 0; i < 60; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const ck = await api('/login/qr/check', 'key=' + key + '&t=' + Date.now());
    const s = ck.data?.status;
    console.log('[' + (i+1) + '] s=' + s);
    if (s === 4) { token = ck.data.token; userid = ck.data.userid; console.log('OK uid=' + userid); break; }
    if (s === 0) { console.log('过期'); process.exit(1); }
  }
  if (!token) { console.log('超时'); process.exit(1); }

  const auth = 'userid=' + userid + '&token=' + token;

  // 歌单
  const pl = await api('/user/playlist', auth + '&t=' + Date.now());
  console.log('\n歌单 err:', pl.error_code, 'count:', (pl.data?.info||[]).length);
  const list = pl.data?.info || [];
  list.forEach(p => console.log(`  ${p.name} lid=${p.list_create_listid} mc=${p.m_count}`));

  // 歌单歌曲（我喜欢）
  const like = list.find(p => p.m_count > 0) || list[0];
  const ts = await api('/playlist/track/all/new', auth + '&listid=' + like.list_create_listid + '&pagesize=3&page=1&t=' + Date.now());
  const songs = ts.data?.data || ts.data?.info || [];
  console.log('\n歌曲 err:', ts.error_code, 'count:', songs.length);
  const song = songs[0];
  if (!song) { console.log('无歌曲'); process.exit(1); }
  console.log('第一首:', (song.songname||song.name), 'hash=' + song.hash);

  // 播放
  const play = await api('/song/url', auth + '&hash=' + song.hash + '&album_id=' + (song.album_id||0) + '&album_audio_id=' + (song.album_audio_id||0) + '&t=' + Date.now());
  const url = play.data?.url || play.url;
  if (url) {
    console.log('\n✅ PLAY URL: ' + url.substring(0, 100));
    process.exit(0);
  }
  console.log('\nerrcode:', play.errcode, 'error_code:', play.error_code);
  if (play.errcode === 20028) {
    console.log('SSA! eventid=' + (play.edt||'').substring(0, 30));
    const vi = await api('/get/verify/info', auth + '&eventid=' + encodeURIComponent(play.edt) + '&t=' + Date.now());
    console.log('get_verify_info: ' + JSON.stringify(vi, null, 2));
    if (vi.data?.captcha_url) console.log('\n验证码URL: ' + vi.data.captcha_url);
  } else {
    console.log(JSON.stringify(play).substring(0, 500));
  }
})().catch(e => console.log('ERR: ' + e.message));
