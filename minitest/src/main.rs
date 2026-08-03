use std::io::Cursor;
use std::sync::mpsc;
use std::time::Duration;

fn main() {
    println!("=== 测试5: 新鲜 URL 测试 ===");

    let (tx, rx) = mpsc::channel::<String>();

    std::thread::spawn(move || {
        let (_stream, handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&handle).unwrap();
        println!("[Player] 设备 OK");

        loop {
            println!("[Player] 等待...");
            let msg = match rx.recv() {
                Ok(m) => m,
                Err(_) => { println!("[Player] 通道关闭"); return; }
            };

            if msg.starts_with("URL:") {
                let url = msg[4..].to_string();
                println!("[Player] 收到 URL，开始下载...");
                
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .unwrap();
                
                match client.get(&url).send() {
                    Ok(r) => {
                        println!("[Player] HTTP {}", r.status().as_u16());
                        match r.bytes() {
                            Ok(data) => {
                                println!("[Player] 下载完成 {} 字节", data.len());
                                let cursor = Cursor::new(data.to_vec());
                                match rodio::Decoder::new(cursor) {
                                    Ok(source) => {
                                        sink.append(source);
                                        println!("[Player] 已 append，播放中...");
                                    }
                                    Err(e) => println!("[Player] 解码失败: {}", e),
                                }
                            }
                            Err(e) => println!("[Player] 读取失败: {}", e),
                        }
                    }
                    Err(e) => println!("[Player] 下载失败: {}", e),
                }
            }
        }
    });

    std::thread::sleep(Duration::from_millis(300));
    
    // ★ 刚刚从日志抓的新鲜 URL
    let url = r"http://kw-lv.kuwo.cn/aa13a3b7984fb4469c756fb8b3d46cd0/6a670b34/resource/30106/trackmedia/M800003CBcYo1ZpG7V.mp3?bitrate$320&format$mp3&source$&type$convert_url_with_sign&user$C_APK_guanwang_178513797119858000015796&loginUid$";
    
    println!("[主] 发送新鲜 URL");
    tx.send(format!("URL:{}", url)).unwrap();

    println!("[主] 等 10 秒...");
    std::thread::sleep(Duration::from_secs(10));
    println!("[主] 完成");
}
