process.env.NODE_PATH = 'D:/workspaces/search/projects/midou-music-v2/kuGouMusicApi';
require('module').Module._initPaths();
const { createRequest } = require('util/request.js');
const songUrl = require('module/song_url_new.js');
const songUrlV5 = require('module/song_url.js');

const cookie = {
  dfid: '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6',
  token: '167879a042c34f685445eb5e0db12472fbaec140aeffd424803fd7a8d74e7c6c',
  userid: '1514557990',
  KUGOU_API_MID: '4b0a5cb94103098b612eecd6f9d4cc08',
};

async function test(name, fn, params) {
  try {
    const result = await fn(params, createRequest);
    console.log('[' + name + '] STATUS: ' + result.status);
    console.log('[' + name + '] BODY: ' + JSON.stringify(result.body).substring(0, 500));
  } catch(e) {
    console.error('[' + name + '] ERROR: ' + (e.message || e));
  }
}

async function main() {
  await test('v6', songUrl, { hash: 'b5f2062d47a40b94a0912d4d48439e59', cookie });
  await test('v5', songUrlV5, { hash: 'b5f2062d47a40b94a0912d4d48439e59', cookie, album_audio_id: '462211566' });
  await test('v6_free', songUrl, { hash: '212d9d7da2410b7e831737e2ba951ed2', cookie });
}
main();
