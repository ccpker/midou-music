import json, base64
d = json.load(open('qr2.json'))
img = d['data']['qrcode_img'].split(',')[1]
open('qr.png', 'wb').write(base64.b64decode(img))
print('key:', d['data']['qrcode'])
