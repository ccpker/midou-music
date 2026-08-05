#!/usr/bin/env python3
"""
生成米豆音乐图标 - 使用 lucide Music 图标风格
"""
from PIL import Image, ImageDraw

def generate_icon(size=128):
    """生成单个尺寸的图标"""
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # 背景圆（渐变蓝紫色调）
    center = size // 2
    radius = size // 2 - 2
    draw.ellipse([2, 2, size-2, size-2], fill=(99, 102, 241, 255))  # indigo-500
    
    # 音乐符号（简化的音符形状）
    note_color = (255, 255, 255, 255)  # 白色
    note_scale = size / 128
    
    # 音符主体（圆形 + 竖线 + 旗帜）
    # 圆形底部
    note_x = int(35 * note_scale)
    note_y = int(85 * note_scale)
    note_r = int(12 * note_scale)
    draw.ellipse([
        note_x - note_r, note_y - note_r,
        note_x + note_r, note_y + note_r
    ], fill=note_color)
    
    # 竖线
    line_x = note_x + note_r
    line_top = int(30 * note_scale)
    line_bottom = note_y
    line_width = int(4 * note_scale)
    draw.rectangle([
        line_x - line_width//2, line_top,
        line_x + line_width//2, line_bottom
    ], fill=note_color)
    
    # 旗帜（弧形简化为三角形）
    flag_points = [
        (line_x, line_top),
        (int(85 * note_scale), int(45 * note_scale)),
        (line_x, int(55 * note_scale))
    ]
    draw.polygon(flag_points, fill=note_color)
    
    # 第二个音符（稍小，偏右）
    note2_x = int(70 * note_scale)
    note2_y = int(95 * note_scale)
    note2_r = int(10 * note_scale)
    draw.ellipse([
        note2_x - note2_r, note2_y - note2_r,
        note2_x + note2_r, note2_y + note2_r
    ], fill=note_color)
    
    # 第二个竖线
    line2_x = note2_x + note2_r
    draw.rectangle([
        line2_x - line_width//2, int(40 * note_scale),
        line2_x + line_width//2, note2_y
    ], fill=note_color)
    
    return img

def main():
    import os
    
    # 输出目录 - 项目根目录下的 src-tauri/icons
    script_dir = os.path.dirname(os.path.abspath(__file__))
    icons_dir = os.path.join(os.path.dirname(script_dir), "src-tauri", "icons")
    
    # 生成各尺寸 PNG
    sizes = [32, 128, 256]
    for size in sizes:
        img = generate_icon(size)
        filename = f"{size}x{size}.png" if size != 256 else "128x128@2x.png"
        img.save(os.path.join(icons_dir, filename))
        print(f"生成: {filename}")
    
    # 生成 icon.png (512x512)
    img = generate_icon(512)
    img.save(os.path.join(icons_dir, "icon.png"))
    print("生成: icon.png (512x512)")
    
    # 生成 ico (Windows)
    sizes_ico = [16, 32, 48, 64, 128, 256]
    images = [generate_icon(s) for s in sizes_ico]
    images[0].save(
        os.path.join(icons_dir, "icon.ico"),
        format='ICO',
        sizes=[(s, s) for s in sizes_ico],
        append_images=images[1:]
    )
    print("生成: icon.ico")
    
    # 生成 icns (macOS) - 需要 png2icns 或手动
    # 这里只生成 PNG，macOS 需要单独处理
    print("\n✅ 图标生成完成！")
    print("macOS .icns 需要手动生成或使用其他工具")

if __name__ == "__main__":
    main()
