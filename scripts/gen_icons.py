import struct, zlib, os

def make_png(width, height, r, g, b):
    def chunk(ctype, data):
        c = ctype + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    header = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
    raw = b''
    for y in range(height):
        raw += b'\x00' + bytes([r, g, b]) * width
    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')
    return header + ihdr + idat + iend

base = r'D:\workspaces\search\projects\midou-music\src-tauri\icons'
os.makedirs(base, exist_ok=True)

with open(os.path.join(base, '32x32.png'), 'wb') as f:
    f.write(make_png(32, 32, 59, 130, 246))
with open(os.path.join(base, '128x128.png'), 'wb') as f:
    f.write(make_png(128, 128, 59, 130, 246))
with open(os.path.join(base, '128x128@2x.png'), 'wb') as f:
    f.write(make_png(256, 256, 59, 130, 246))

# Minimal .ico (32x32, 4 bytes/pixel)
import struct as st
width, height = 32, 32
xor_size = ((width * height * 4) + 40)  # bitmap header
and_size = ((width + 7) // 8 * 4) * height
file_header = st.pack('<HHHHHII', 0, 1, 1, width, height, 0, xor_size + and_size, xor_size)
# BITMAPINFOHEADER
bmp_header = st.pack('<IiiHHIIiiII', 40, width, height * 2, 1, 32, 0, width * height * 4, 0, 0, 0, 0)
# pixel data: BGRA
pixels = b''
for y in range(height - 1, -1, -1):
    for x in range(width):
        pixels += bytes([246, 130, 59, 255])  # B=59 G=130 R=246

with open(os.path.join(base, 'icon.ico'), 'wb') as f:
    f.write(file_header + bmp_header + pixels)

# placeholder icns
with open(os.path.join(base, 'icon.icns'), 'wb') as f:
    f.write(b'\x00')

print(f'Icons created in {base}')
