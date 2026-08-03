// 单独生成二维码图片
process.env.platform = 'lite';
const api = require('./main');
const fs = require('fs');

(async () => {
  const qrKey = await api.login_qr_key();
  const key = qrKey.body.data.qrcode;
  const b64 = qrKey.body.data.qrcode_img.split(',')[1];
  fs.writeFileSync(__dirname + '/qr.png', Buffer.from(b64, 'base64'));
  console.log('QR key:', key);
  console.log('QR 图片已保存到 qr.png');
  console.log('URL: https://h5.kugou.com/apps/loginQRCode/html/index.html?qrcode=' + key);
})();
