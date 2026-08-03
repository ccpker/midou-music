const crypto = require('crypto');
const md5 = (s) => crypto.createHash('md5').update(s).digest('hex');

const salt = 'OIlwieks28dk2k092lksi2UIkp';
const paramsStr = 'appid=1005clienttime=1785389144clientver=20489dfid=2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6mid=4b0a5cb94103098b612eecd6f9d4cc08token=167879a042c34f685445eb5e0db12472fbaec140aeffd424803fd7a8d74e7c6cuserid=1514557990uuid=-';
const body = '{"area_code":"1","behavior":"play","qualities":["128","320","flac","high","multitrack"],"resource":{"collect_list_id":"3","collect_time":1785389144696,"hash":"b5f2062d47a40b94a0912d4d48439e59","id":0,"page_id":1,"type":"audio"},"token":"167879a042c34f685445eb5e0db12472fbaec140aeffd424803fd7a8d74e7c6c","tracker_param":{"all_m":1,"auth":"","is_free_part":0,"key":"369c6105274a9f976d8a8a30ee4b444e","module_id":0,"need_climax":1,"need_xcdn":1,"open_time":"","pid":"411","pidversion":"3001","priv_vip_type":"6","viptoken":""},"userid":"1514557990","vip":0}';

const expected = '61b50eb521f482abead5ec304a03080e';
const sig = md5(salt + paramsStr + body + salt);
console.log('Expected:', expected);
console.log('Got:     ', sig);
console.log('Match:   ', sig === expected);

// Also try the Node.js helper directly
const { signatureAndroidParams } = require('D:/workspaces/search/projects/midou-music/tools/KuGouMusicApi/util/helper.js');
const params = {
  appid: '1005', clienttime: '1785389144', clientver: '20489',
  dfid: '2-a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6', mid: '4b0a5cb94103098b612eecd6f9d4cc08',
  token: '167879a042c34f685445eb5e0db12472fbaec140aeffd424803fd7a8d74e7c6c',
  userid: '1514557990', uuid: '-'
};
const sig2 = signatureAndroidParams(params, body);
console.log('Helper:  ', sig2);
console.log('Match2:  ', sig2 === expected);
