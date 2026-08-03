// 生成QR → 你扫码 → 立刻全链路验证
const http = require('http');
const fs = require('fs');

function api(path, qs) {
  return new Promise((resolve, reject) => {
    http.get(`http://localhost:3000${path}?${qs}`, res => {
      let d = ''; res.on('data', c => d += c); res.on('end', () => {
        try { resolve(JSON.parse(d)); } catch(e) { reject(e); }
      });
    }).on('error', reject);
  });
}

(async () => {
  // 取QR key + 存图
  const ts = Date.now();
  const qr = await api('/login/qr/key', 'timestamp=' + ts);
  const key = qr.data?.qrcode;
  const imgB64 = qr.data?.qrcode_img?.split(',')[1];
  if (imgB64) fs.writeFileSync(__dirname + '/qr.png', Buffer.from(imgB64, 'base64'));
  console.log('扫 qr.png！key=' + key);

  // 轮询扫码
  let token, userid;
  for (let i = 0; i < 60; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const ck = await api('/login/qr/check', 'key=' + key + '&timestamp=' + Date.now());
    const s = ck.data?.status;
    console.log(`[${i+1}] status=${s}`);
    if (s === 4) { token = ck.data?.token; userid = ck.data?.userid; console.log('✅ 登录 userid=' + userid); break; }
    if (s === 0) { console.log('过期！'); process.exit(1); }
  }
  if (!token) { console.log('超时'); process.exit(1); }

  const auth = 'userid=' + userid + '&token=' + token;

  // 搜索
  const sr = await api('/search', auth + '&keyword=%E6%99%B4%E5%A4%A9&pagesize=1&timestamp=' + Date.now());
  const song = (sr.data?.data || [])[0];
  if (!song) { console.log('搜索无结果'); process.exit(1); }
  console.log('\n搜索:', song.songname, song.hash);

  // 播放
  const play = await api('/song/url', auth + '&hash=' + song.hash + '&album_id=' + (song.album_id||0) + '&album_audio_id=' + (song.album_audio_id||0) + '&timestamp=' + Date.now());
  console.log('\n播放响应:');
  console.log('errcode:', play.errcode, 'error_code:', play.error_code);
  const url = play.data?.url || play.url;
  if (url) {
    console.log('✅ ' + url.substring(0, 120));
  } else if (play.errcode === 20028) {
    console.log('20028 SSA验证!');
    const eventid = play.edt;
    console.log('eventid:', eventid?.substring(0, 40));

    // get_verify_info
    const vi = await api('/get/verify/info', auth + '&eventid=' + encodeURIComponent(eventid) + '&timestamp=' + Date.now());
    console.log('\nget_verify_info:');
    console.log(JSON.stringify(vi, null, 2));
    if (vi.data?.captcha_url) console.log('\n验证码URL:', vi.data.captcha_url);
  } else {
    console.log('unknown:', JSON.stringify(play).substring(0, 500));
  }
})().catch(e => console.log('ERR:', e.message));
