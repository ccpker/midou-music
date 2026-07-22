"""Generate a 1240x1240 placeholder PNG for tauri icon."""
import struct, zlib, os

def make_png(width, height, r, g, b):
    def chunk(ctype, data):
        c = ctype + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)
    header = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
    raw = b''
    for _ in range(height):
        raw += b'\x00' + bytes([r, g, b]) * width
    idat = chunk(b'IDAT', zlib.compress(raw))
    iend = chunk(b'IEND', b'')
    return header + ihdr + idat + iend

base = r'D:\workspaces\search\projects\midou-music'
png_path = os.path.join(base, 'app-icon.png')
with open(png_path, 'wb') as f:
    f.write(make_png(1240, 1240, 59, 130, 246))
print(f'Created {png_path} ({os.path.getsize(png_path)} bytes)')
