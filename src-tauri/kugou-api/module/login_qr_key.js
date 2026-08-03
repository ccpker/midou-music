const { srcappid, appid } = require('../util');

// 二维码 key 生成接口
module.exports = (params, useAxios) => {
  return useAxios({
    baseURL: 'https://login-user.kugou.com',
    url: '/v2/qrcode',
    method: 'GET',
    params: {
      appid: params?.type === 'web' ? 1014 : appid,  // 使用平台 appid（lite=3116 / 标准=1005）
      type: 1,
      plat: 4,
      qrcode_txt: `https://h5.kugou.com/apps/loginQRCode/html/index.html?appid=${appid}&`,
      srcappid,
    },
    encryptType: 'web',
    cookie: params?.cookie || {},
  });
};
