// ════════════════════════════════════════════════
// 音楽自由 — 纯净搜索(三层组合)
// ────────────────────────────────────────────
// 功能:   咪咕+MusicBrainz+酷我三源组合搜索,精确定位原唱
// 输入:   关键字 + 分页参数
// 输出:   Vec<Song>(含score排序)
// 依赖:   search::migu, search::kuwo, search::musicbrainz
// 备注:   五维评分确保原唱排在前面; 三级降级链防止无结果
// ════════════════════════════════════════════════

use reqwest::Client;

use crate::models::Song;
use crate::search::{kuwo, migu, musicbrainz};
use crate::log_info;

/// 纯净搜索(组合策略)
///
/// # 过程
/// 1. 咪咕搜索 → 拿干净歌名+歌手+版权标识
/// 2. 并行: 酷我精确搜索 + MusicBrainz MBID查证
/// 3. 三级降级链(精确→歌手过滤→咪咕直取)
/// 4. 五维评分排序(歌名/歌手/版权/时长/原唱)
///
/// # 返回
/// - `Ok(Vec<Song>)`: 已排序的结果,score字段含评分
pub async fn search(client: &Client, keyword: &str, page: usize, page_size: usize) -> Result<Vec<Song>, String> {
    // ── Step 1: 咪咕搜索(主),拿干净歌名+歌手 ──
    let migu_songs = migu::search(client, keyword, 0, 8).await.unwrap_or_default();

    if migu_songs.is_empty() {
        log_info!("search", "纯净: 咪咕无结果,降级普通搜索");
        return kuwo::search(client, keyword, page, page_size).await;
    }

    let top_name = migu_songs[0].name.clone();
    let top_singer = migu_songs[0].singer.clone();
    let top_dur = migu_songs[0].duration;
    let top_copyright = migu_songs[0].migu_copyright.clone();

    log_info!("search", "migu top: name={} singer={} dur={} copyright={:?}",
        top_name, top_singer, top_dur, top_copyright);

    let target_name = top_name.trim();
    let target_artist = if !top_singer.trim().is_empty() {
        top_singer.trim().to_string()
    } else if keyword.contains(' ') {
        keyword.split_whitespace().skip(1).collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    };
    let target_dur = top_dur;
    log_info!("search", "纯净目标: {} - {} (估算{}s)", target_name, target_artist, target_dur);

    // ── Step 2: 并行——酷我搜索(精确关键词) + MB时长查证 ──
    let precise_kw = format!("{} {}", target_name, target_artist);
    log_info!("search", "纯净精确搜索: {}", precise_kw);

    let mb_fut = musicbrainz::search(client, target_name, &target_artist);
    let kw_fut = kuwo::search(client, &precise_kw, page, page_size);
    let (mb_result, kw_songs) = tokio::join!(mb_fut, kw_fut);
    let mut songs = kw_songs.unwrap_or_default();

    // ── Step 3: 三级降级链 ──
    if songs.is_empty() && !target_artist.is_empty() {
        // L2: 搜歌手名,从专辑中过滤目标歌名
        log_info!("search", "L1 无结果, L2 降级: 搜歌手名 \"{}\"", target_artist);
        if let Ok(artist_songs) = kuwo::search(client, &target_artist, 0, 30).await {
            let clean_n = target_name.trim().to_lowercase();
            songs = artist_songs.into_iter()
                .filter(|s| {
                    let sn = s.name.trim().to_lowercase();
                    sn == clean_n
                        || sn.starts_with(&format!("{} ", clean_n))
                        || sn.starts_with(&format!("{}（", clean_n))
                        || sn.starts_with(&format!("{}(", clean_n))
                })
                .collect();
            if !songs.is_empty() {
                log_info!("search", "L2 命中 {} 条", songs.len());
            }
        }
    }

    if songs.is_empty() && !migu_songs.is_empty() {
        // L3: 咪咕直取(仅展示,须标记来源)
        log_info!("search", "L2 无结果, L3 降级: 咪咕直取 {} 首", migu_songs.len());
        songs = migu_songs.into_iter()
            .filter(|s| {
                let sn = s.name.trim().to_lowercase();
                let has_medley = sn.contains("dj") || sn.contains("remix")
                    || sn.contains("翻唱") || sn.contains("翻奏") || sn.contains("伴奏")
                    || sn.contains("串烧") || sn.contains("cover") || sn.contains("live")
                    || sn.contains("medley");
                !has_medley
            })
            .collect();
        log_info!("search", "L3 过滤后 {} 首", songs.len());
    }

    // ── Step 4: MB 结果标注 ──
    let mb_dur = mb_result.ok().flatten().map(|(_, d)| d);
    let mb_found = mb_dur.is_some();
    log_info!("search", "MB found={}, duration={:?}", mb_found, mb_dur);

    // ── Step 5: 五维评分 ──
    let target_artists: Vec<&str> = target_artist
        .split(|c| c == '&' || c == '&' || c == ',' || c == ',' || c == '/' || c == ' ')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let is_original = top_copyright.as_deref() == Some("1");
    log_info!("search", "目标: {} | 原唱={} | target_artists={:?}",
        target_name, is_original, target_artists);

    for song in &mut songs {
        let mut sc: u32 = 0;
        let song_name = song.name.trim();
        let singer_clean = song.singer.replace("\\&", "&");
        let song_artists: Vec<&str> = singer_clean
            .split(|c| c == '&' || c == '&' || c == ',' || c == ',' || c == '/' || c == ' ')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // 维度1: 歌名匹配(+30/+10/+0)
        if song_name == target_name {
            sc += 30;
        } else if song_name.starts_with(target_name) && song_name.len() > target_name.len() {
            sc += 10;
        }

        // 维度2: 歌手匹配(+20/+5/−10)
        let artist_exact = !target_artists.is_empty() && !song_artists.is_empty()
            && target_artists.iter().all(|ta| song_artists.contains(ta));
        let artist_partial = target_artists.iter()
            .any(|ta| !ta.is_empty() && song_artists.contains(ta));

        if artist_exact {
            sc += 20;
        } else if artist_partial {
            sc += 5;
        } else if !target_artists.is_empty() {
            sc = sc.saturating_sub(10);
        }

        // 维度3: 版权纯净分(+15)
        if song.migu_copyright.as_deref() == Some("1") {
            sc += 15;
        }

        // 维度4: 时长加权(+5)
        if song.duration >= 200 && song.duration <= 260 {
            sc += 5;
        }

        // 维度5: 原唱兜底(+20)
        let has_suffix = song_name != target_name;
        if !has_suffix && artist_exact && is_original {
            sc += 20;
        }

        // 翻唱/DJ/伴奏惩罚(−40)
        let medley_kw = ["翻唱", "翻奏", "dj", "DJ", "remix", "Remix", "伴奏", "串烧",
            "cover", "Cover", "live", "Live", "medley", "Medley", "ktv", "KTV"];
        let is_medley = medley_kw.iter()
            .any(|kw| song_name.contains(kw) || song.singer.contains(kw));
        if is_medley {
            sc = sc.saturating_sub(40);
        }

        song.score = Some(sc);
        song.migu_duration = Some(target_dur);
    }

    // 按评分降序
    songs.sort_by(|a, b| b.score.unwrap_or(0).cmp(&a.score.unwrap_or(0)));
    log_info!("search", "纯净返回 {} 首", songs.len());
    Ok(songs)
}
