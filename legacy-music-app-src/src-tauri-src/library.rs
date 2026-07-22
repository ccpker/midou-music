// ════════════════════════════════════════════════
// 音楽自由 — 库管理核心模块 (Phase 3)
// ────────────────────────────────────────────
// 功能:   多音乐库管理、文件夹树、回收站、歌词管理
// 输入:   IPC 命令参数 + data/libraries-index.json
// 输出:   LibrariesIndex 持久化 + lrc 文件 + 文件操作
// 依赖:   serde, serde_json, uuid, chrono, std::fs
// 边界:   数据目录与音乐库目录分离; 删库不动物理文件
// 备注:   歌词存 data/lyrics/{music_id}.lrc，不存音乐库目录
// ════════════════════════════════════════════════

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

pub use tauri::State as TauriState;

// ── 数据结构 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrariesIndex {
    pub libraries: Vec<Library>,
    pub songs: HashMap<String, SongEntry>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: String,           // uuid
    pub name: String,         // 库名
    pub path: String,         // 音乐库目录路径（任意位置）
    pub is_master: bool,      // 是否主库
    pub active: bool,         // 当前激活
    pub folders: Vec<Folder>, // 文件夹树
    pub trash: Vec<TrashEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongEntry {
    pub music_id: String,     // mid_kuwo_123456
    pub title: String,
    pub artist: String,
    pub source: String,       // "kuwo" | "bilibili" | "local"
    pub source_id: String,    // 源ID
    pub file_path: String,    // 库目录下的相对路径 "周杰伦/七里香.mp3"
    pub file_size: u64,
    pub lyrics_edited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub song_ids: Vec<String>, // music_id 列表
    #[serde(default)]
    pub system: bool,          // true=系统文件夹（P8-3：不可删/不可改名）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashEntry {
    pub music_id: String,
    pub deleted_at: String,     // ISO 时间戳
    pub original_path: String,  // 回收前的文件路径
}

// ── 持久化路径 ──

/// 获取程序 data 目录 (与下载目录无关)
pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("music-app")
}

fn index_path() -> PathBuf {
    data_dir().join("libraries-index.json")
}

fn index_bak_path() -> PathBuf {
    data_dir().join("libraries-index.json.bak")
}

fn lyrics_dir() -> PathBuf {
    data_dir().join("lyrics")
}

fn trash_dir() -> PathBuf {
    data_dir().join("trash")
}

fn lyrics_path(music_id: &str) -> PathBuf {
    lyrics_dir().join(format!("{music_id}.lrc"))
}

// ── 加载/保存 ──

/// 加载库索引 (不存在 → 空默认)
pub fn load_libraries_index() -> LibrariesIndex {
    let path = index_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(idx) = serde_json::from_str::<LibrariesIndex>(&data) {
            crate::log_info!("library", "加载库索引: {} 个库, {} 首歌", idx.libraries.len(), idx.songs.len());
            return idx;
        }
    }
    LibrariesIndex {
        libraries: vec![],
        songs: HashMap::new(),
        version: 1,
    }
}

/// 保存库索引 + 备份
pub fn save_libraries_index(idx: &LibrariesIndex) {
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);

    if let Ok(s) = serde_json::to_string_pretty(idx) {
        let path = index_path();
        // 先备份再写入
        let _ = fs::copy(&path, index_bak_path());
        let _ = fs::write(&path, &s);
    }
}

// ── 工具函数 ──

/// 查找激活库
pub fn active_library(libs: &[Library]) -> Option<&Library> {
    libs.iter().find(|l| l.active)
}

/// 查找激活库 (mutable)
pub fn active_library_mut(libs: &mut [Library]) -> Option<&mut Library> {
    libs.iter_mut().find(|l| l.active)
}

/// 建 music_id
pub fn make_music_id(source: &str, source_id: &str) -> String {
    format!("mid_{source}_{source_id}")
}

/// 找歌 (by music_id)
pub fn find_song<'a>(lib: &'a Library, songs: &'a HashMap<String, SongEntry>, music_id: &str) -> Option<&'a SongEntry> {
    songs.get(music_id).filter(|_s| lib.songs_contain(music_id))
}

impl Library {
    pub fn songs_contain(&self, music_id: &str) -> bool {
        self.folders.iter().any(|f| f.song_ids.iter().any(|id| id == music_id))
    }

    pub fn trash_contain(&self, music_id: &str) -> bool {
        self.trash.iter().any(|t| t.music_id == music_id)
    }

    /// 查找文件夹
    pub fn find_folder(&self, folder_id: &str) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == folder_id)
    }

    pub fn find_folder_mut(&mut self, folder_id: &str) -> Option<&mut Folder> {
        self.folders.iter_mut().find(|f| f.id == folder_id)
    }
}

// ── IPC 命令实现 ──

type LibState = Arc<Mutex<LibrariesIndex>>;

/// 1) 新建库
#[tauri::command]
pub fn create_library(state: TauriState<'_, LibState>, name: String, path: String) -> Result<Library, String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    // 路径不能重复
    if idx.libraries.iter().any(|l| l.path == path) {
        return Err("该路径已关联到其他库".into());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let default_folder = Folder {
        id: uuid::Uuid::new_v4().to_string(),
        name: "默认".to_string(),
        parent_id: None,
        song_ids: vec![],
        system: false,
    };

    // P8-3: 系统源文件夹（每个库自动创建，不可删）
    let system_folders = vec![
        ("♬ 酷我", "kuwo"),
        ("♬ 酷狗", "kugou"),
        ("♬ B站", "bilibili"),
        ("♬ 本地", "local"),
    ];
    let mut all_folders = vec![default_folder];
    for (name, _source) in &system_folders {
        all_folders.push(Folder {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            parent_id: None,
            song_ids: vec![],
            system: true,
        });
    }

    let lib = Library {
        id,
        name,
        path,
        is_master: idx.libraries.is_empty(), // 第一个库默认主库
        active: idx.libraries.is_empty(),    // 第一个库默认激活
        folders: all_folders,
        trash: vec![],
    };

    idx.libraries.push(lib.clone());
    save_libraries_index(&idx);

    crate::log_info!("library", "新建库: {} ({})", lib.name, lib.id);
    Ok(lib)
}

/// 2) 切换激活库
#[tauri::command]
pub fn switch_library(state: TauriState<'_, LibState>, library_id: String) -> Result<Library, String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    // 取消所有激活
    for l in &mut idx.libraries {
        l.active = false;
    }

    // 激活目标库
    let lib = idx.libraries.iter_mut()
        .find(|l| l.id == library_id)
        .ok_or("库不存在")?;

    lib.active = true;
    let result = lib.clone();
    save_libraries_index(&idx);

    crate::log_info!("library", "切换激活库: {} ({})", result.name, result.id);
    Ok(result)
}

/// 3) 删库 (从索引移除，不动物理文件)
#[tauri::command]
pub fn delete_library(state: TauriState<'_, LibState>, library_id: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    let pos = idx.libraries.iter().position(|l| l.id == library_id)
        .ok_or("库不存在")?;

    let lib = &idx.libraries[pos];

    // 清理关联的歌词文件 (文件小，直接删)
    for music_id in idx.songs.keys() {
        if lib.songs_contain(music_id) || lib.trash_contain(music_id) {
            let _ = fs::remove_file(lyrics_path(music_id));
        }
    }

    // 清理 songs map 中仅属于本库的条目
    let music_ids_to_keep: Vec<String> = idx.songs.keys()
        .filter(|mid| {
            idx.libraries.iter().enumerate().any(|(i, l)| {
                i != pos && (l.songs_contain(mid) || l.trash_contain(mid))
            })
        })
        .cloned()
        .collect();

    idx.songs.retain(|k, _| music_ids_to_keep.iter().any(|mid| mid == k));
    idx.libraries.remove(pos);

    save_libraries_index(&idx);
    crate::log_info!("library", "删除库: {library_id} (文件未动)");
    Ok(())
}

/// 4) 合并库
#[tauri::command]
pub fn merge_library(state: TauriState<'_, LibState>, from_id: String, to_id: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    if from_id == to_id {
        return Err("不能合并到自己".into());
    }

    let from_pos = idx.libraries.iter().position(|l| l.id == from_id)
        .ok_or("源库不存在")?;
    let to_pos = idx.libraries.iter().position(|l| l.id == to_id)
        .ok_or("目标库不存在")?;

    // 验路径
    let from_path = idx.libraries[from_pos].path.clone();
    let to_path = idx.libraries[to_pos].path.clone();

    // 物理复制文件
    let music_ids: Vec<String> = idx.libraries[from_pos]
        .folders.iter()
        .flat_map(|f| f.song_ids.clone())
        .collect();

    let mut copied = 0u32;
    for mid in &music_ids {
        if let Some(song) = idx.songs.get(mid) {
            let src = PathBuf::from(&from_path).join(&song.file_path);
            let dst = PathBuf::from(&to_path).join(&song.file_path);
            if src.exists() {
                if let Some(parent) = dst.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&src, &dst).is_ok() {
                    copied += 1;
                }
            }
        }
    }

    // 合并数据
    let from_lib = &idx.libraries[from_pos];
    let from_folders = from_lib.folders.clone();
    let from_trash = from_lib.trash.clone();

    // 把源库的文件夹 + 歌移到目标库
    let _default_folder_id = idx.libraries[to_pos].folders
        .iter()
        .find(|f| f.name == "默认")
        .map(|f| f.id.clone())
        .unwrap_or_else(|| {
            let id = uuid::Uuid::new_v4().to_string();
            idx.libraries[to_pos].folders.push(Folder {
                id: id.clone(),
                name: "默认".to_string(),
                parent_id: None,
                song_ids: vec![],
                system: false,
            });
            id
        });

    for f in from_folders {
        let mut existing = idx.libraries[to_pos].find_folder_mut(&f.id);
        if let Some(ref mut ef) = existing {
            for sid in &f.song_ids {
                if !ef.song_ids.contains(sid) {
                    ef.song_ids.push(sid.clone());
                }
            }
        } else {
            idx.libraries[to_pos].folders.push(f);
        }
    }

    // 合并垃圾
    for t in from_trash {
        if !idx.libraries[to_pos].trash.iter().any(|tt| tt.music_id == t.music_id) {
            idx.libraries[to_pos].trash.push(t);
        }
    }

    // 移除源库
    idx.libraries.remove(from_pos);
    save_libraries_index(&idx);

    crate::log_info!("library", "合并库: {from_id} → {to_id}, 复制 {copied} 个文件");
    Ok(())
}

/// 5) 列出所有库
#[tauri::command]
pub fn list_libraries(state: TauriState<'_, LibState>) -> Result<Vec<Library>, String> {
    let idx = state.lock().map_err(|e| e.to_string())?;
    Ok(idx.libraries.clone())
}

/// 6) 设主库
#[tauri::command]
pub fn set_master_library(state: TauriState<'_, LibState>, library_id: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    for l in &mut idx.libraries {
        l.is_master = false;
    }

    let lib = idx.libraries.iter_mut()
        .find(|l| l.id == library_id)
        .ok_or("库不存在")?;

    lib.is_master = true;
    save_libraries_index(&idx);

    crate::log_info!("library", "设主库: {library_id}");
    Ok(())
}

/// 7) 歌归到文件夹
#[tauri::command]
pub fn add_song_to_folder(state: TauriState<'_, LibState>, music_id: String, folder_id: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;
    let lib = active_library_mut(&mut idx.libraries)
        .ok_or("没有激活的库")?;

    // 从旧文件夹移除
    for f in &mut lib.folders {
        f.song_ids.retain(|id| id != &music_id);
    }

    // 加入新文件夹
    let target = lib.find_folder_mut(&folder_id)
        .ok_or("文件夹不存在")?;

    if !target.song_ids.contains(&music_id) {
        target.song_ids.push(music_id.clone());
    }

    save_libraries_index(&idx);
    crate::log_info!("library", "移动歌曲 {music_id} → 文件夹 {folder_id}");
    Ok(())
}

/// 8) 新建文件夹
#[tauri::command]
pub fn create_folder(state: TauriState<'_, LibState>, name: String, parent_id: Option<String>) -> Result<Folder, String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;
    let lib = active_library_mut(&mut idx.libraries)
        .ok_or("没有激活的库")?;

    // 同名检查
    if lib.folders.iter().any(|f| f.name == name && f.parent_id == parent_id) {
        return Err("同名文件夹已存在".into());
    }

    let folder = Folder {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        parent_id,
        song_ids: vec![],
        system: false,
    };

    lib.folders.push(folder.clone());
    save_libraries_index(&idx);

    crate::log_info!("library", "新建文件夹: {}", folder.name);
    Ok(folder)
}

/// 9) 删空文件夹 (非空不删)
#[tauri::command]
pub fn delete_folder(state: TauriState<'_, LibState>, folder_id: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;
    let lib = active_library_mut(&mut idx.libraries)
        .ok_or("没有激活的库")?;

    let folder = lib.find_folder(&folder_id)
        .ok_or("文件夹不存在")?;

    // P8-3: 系统文件夹保护
    if folder.system {
        return Err("系统文件夹不能删除".into());
    }

    if !folder.song_ids.is_empty() {
        return Err(format!("文件夹不为空 ({} 首歌)，请先移出歌曲", folder.song_ids.len()));
    }

    // 不允许删最后一个文件夹
    if lib.folders.len() <= 1 {
        return Err("不能删除最后一个文件夹".into());
    }

    lib.folders.retain(|f| f.id != folder_id);
    save_libraries_index(&idx);

    crate::log_info!("library", "删除文件夹: {folder_id}");
    Ok(())
}

/// 10) 移到回收站 (物理文件移 data/trash/，索引移 trash 列表)
#[tauri::command]
pub fn move_to_trash(state: TauriState<'_, LibState>, music_id: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    // 先提取 song 数据（clone，避免借用冲突）
    let file_path = idx.songs.get(&music_id)
        .map(|s| s.file_path.clone())
        .ok_or("歌曲不存在")?;

    let lib_path = {
        let lib = active_library_mut(&mut idx.libraries)
            .ok_or("没有激活的库")?;

        // 从所有文件夹移除
        for f in &mut lib.folders {
            f.song_ids.retain(|id| id != &music_id);
        }

        PathBuf::from(&lib.path)
    };

    let src_path = lib_path.join(&file_path);

    // 移到 trash 目录
    let trash = trash_dir();
    let _ = fs::create_dir_all(&trash);
    let trash_file = trash.join(format!("{}_{}", &music_id, file_path.split('/').last().unwrap_or("unknown")));

    if src_path.exists() {
        fs::rename(&src_path, &trash_file)
            .map_err(|e| format!("移动文件失败: {e}"))?;
    }

    let entry = TrashEntry {
        music_id: music_id.clone(),
        deleted_at: chrono::Utc::now().to_rfc3339(),
        original_path: file_path,
    };

    let lib = active_library_mut(&mut idx.libraries)
        .ok_or("没有激活的库")?;
    lib.trash.push(entry);
    save_libraries_index(&idx);

    crate::log_info!("library", "移到回收站: {music_id}");
    Ok(())
}

/// 11) 从回收站恢复
#[tauri::command]
pub fn restore_from_trash(state: TauriState<'_, LibState>, music_id: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;
    let lib = active_library_mut(&mut idx.libraries)
        .ok_or("没有激活的库")?;

    let pos = lib.trash.iter().position(|t| t.music_id == music_id)
        .ok_or("回收站中未找到此歌曲")?;

    let entry = lib.trash[pos].clone();
    let lib_path = PathBuf::from(&lib.path);
    let dst_path = lib_path.join(&entry.original_path);

    // 物理还原
    let trash = trash_dir();
    let trash_file = trash.join(format!("{}_{}", &entry.music_id, entry.original_path.split('/').last().unwrap_or("unknown")));

    if trash_file.exists() {
        if let Some(parent) = dst_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::rename(&trash_file, &dst_path)
            .map_err(|e| format!("还原文件失败: {e}"))?;
    }

    // 归入默认文件夹
    let default_folder = lib.folders.iter_mut()
        .find(|f| f.name == "默认");

    if let Some(f) = default_folder {
        if !f.song_ids.contains(&music_id) {
            f.song_ids.push(music_id.clone());
        }
    }

    lib.trash.remove(pos);
    save_libraries_index(&idx);

    crate::log_info!("library", "从回收站恢复: {music_id}");
    Ok(())
}

/// 12) 清空回收站 (物理删除 trash 目录文件 + 清歌词 + 清索引)
#[tauri::command]
pub fn empty_trash(state: TauriState<'_, LibState>) -> Result<u32, String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    // 先收集要清理的 music_ids（避免借用冲突）
    let trash_ids: Vec<String> = {
        let lib = active_library(&idx.libraries)
            .ok_or("没有激活的库")?;
        lib.trash.iter().map(|t| t.music_id.clone()).collect()
    };

    let count = trash_ids.len() as u32;

    // 1. 物理删除 trash 目录中的文件
    let trash = trash_dir();
    if trash.exists() {
        if let Ok(entries) = fs::read_dir(&trash) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    // 2. 删除关联歌词
    for mid in &trash_ids {
        let lp = lyrics_path(mid);
        let _ = fs::remove_file(&lp);
    }

    // 3. 清理 songs map 中属于 trash 的条目
    for mid in &trash_ids {
        idx.songs.remove(mid);
    }

    // 4. 清空 trash 列表
    let lib = active_library_mut(&mut idx.libraries)
        .ok_or("没有激活的库")?;
    lib.trash.clear();

    save_libraries_index(&idx);

    crate::log_info!("library", "清空回收站: {count} 首歌");
    Ok(count)
}

/// 13) 保存歌词
#[tauri::command]
pub fn save_lyrics(state: TauriState<'_, LibState>, music_id: String, lrc_content: String) -> Result<(), String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    // 确保歌词目录存在
    let dir = lyrics_dir();
    let _ = fs::create_dir_all(&dir);

    // 写入 lrc 文件
    let path = lyrics_path(&music_id);
    fs::write(&path, &lrc_content)
        .map_err(|e| format!("写歌词文件失败: {e}"))?;

    // 标记 edited
    if let Some(song) = idx.songs.get_mut(&music_id) {
        song.lyrics_edited = true;
    }
    save_libraries_index(&idx);

    crate::log_info!("library", "保存歌词: {music_id}");
    Ok(())
}

/// 14) 获取歌词
#[tauri::command]
pub fn get_lyrics(state: TauriState<'_, LibState>, music_id: String) -> Result<String, String> {
    // 不需要锁（只读），但为了保持一致
    let _idx = state.lock().map_err(|e| e.to_string())?;

    let path = lyrics_path(&music_id);
    if path.exists() {
        fs::read_to_string(&path)
            .map_err(|e| format!("读取歌词失败: {e}"))
    } else {
        Ok(String::new())
    }
}

/// 15) 获取当前激活库的完整信息 (含 songs)
#[tauri::command]
pub fn get_active_library(state: TauriState<'_, LibState>) -> Result<serde_json::Value, String> {
    let idx = state.lock().map_err(|e| e.to_string())?;
    let lib = active_library(&idx.libraries)
        .ok_or("没有激活的库")?;

    // 构造前端需要的返回结构
    let folders: Vec<serde_json::Value> = lib.folders.iter().map(|f| {
        let songs: Vec<serde_json::Value> = f.song_ids.iter()
            .filter_map(|mid| idx.songs.get(mid))
            .map(|s| serde_json::json!({
                "music_id": s.music_id,
                "title": s.title,
                "artist": s.artist,
                "source": s.source,
                "source_id": s.source_id,
                "file_path": s.file_path,
                "file_size": s.file_size,
                "lyrics_edited": s.lyrics_edited,
            }))
            .collect();

        serde_json::json!({
            "id": f.id,
            "name": f.name,
            "parent_id": f.parent_id,
            "song_ids": f.song_ids,
            "songs": songs,
        })
    }).collect();

    let trash: Vec<serde_json::Value> = lib.trash.iter().map(|t| {
        let song = idx.songs.get(&t.music_id);
        serde_json::json!({
            "music_id": t.music_id,
            "deleted_at": t.deleted_at,
            "original_path": t.original_path,
            "title": song.map(|s| &s.title),
            "artist": song.map(|s| &s.artist),
        })
    }).collect();

    Ok(serde_json::json!({
        "id": lib.id,
        "name": lib.name,
        "path": lib.path,
        "is_master": lib.is_master,
        "active": lib.active,
        "folders": folders,
        "trash": trash,
    }))
}

/// 16) 入库一首歌曲（从下载/导入时调用）
#[tauri::command]
pub fn add_song_to_library(
    state: TauriState<'_, LibState>,
    title: String,
    artist: String,
    source: String,
    source_id: String,
    file_path: String,
    file_size: u64,
    folder_id: Option<String>,
) -> Result<String, String> {
    let mut idx = state.lock().map_err(|e| e.to_string())?;

    let music_id = make_music_id(&source, &source_id);

    let entry = SongEntry {
        music_id: music_id.clone(),
        title,
        artist,
        source,
        source_id,
        file_path: file_path.clone(),
        file_size,
        lyrics_edited: false,
    };

    let source_for_sys = entry.source.clone();
    idx.songs.insert(music_id.clone(), entry);

    // 放入文件夹
    let target_folder_id = folder_id.unwrap_or_else(|| {
        let lib = active_library(&idx.libraries);
        lib.map(|l| {
            l.folders.iter()
                .find(|f| f.name == "默认")
                .map(|f| f.id.clone())
                .unwrap_or_default()
        }).unwrap_or_default()
    });

    let lib = active_library_mut(&mut idx.libraries)
        .ok_or("没有激活的库")?;
    if let Some(f) = lib.find_folder_mut(&target_folder_id) {
        if !f.song_ids.contains(&music_id) {
            f.song_ids.push(music_id.clone());
        }
    }

    // P8-3: 根据 source 自动归类到系统源文件夹
    if let Some(sys_folder) = lib.folders.iter().find(|f| {
        f.system && f.name.to_lowercase().contains(&source_for_sys)
    }) {
        let sys_id = sys_folder.id.clone();
        let sf = lib.find_folder_mut(&sys_id).unwrap();
        if !sf.song_ids.contains(&music_id) {
            sf.song_ids.push(music_id.clone());
        }
    }

    save_libraries_index(&idx);

    crate::log_info!("library", "入库: {music_id} ← {file_path}");
    Ok(music_id)
}

// ── P8-2: 酷狗收藏同步辅助类型 ──

/// 歌曲导入参数（从酷狗歌单同步时使用）
pub struct SongImport {
    pub title: String,
    pub artist: String,
    pub source: String,
    pub source_id: String,
    pub file_path: String, // 未下载时为空
}

/// P8-2: 同步歌单到激活库（非 Tauri IPC，供 warp 路由直接调用）
/// 返回 (playlist_folder_id, synced_count)
pub fn sync_playlist_to_library(
    libs: &mut LibrariesIndex,
    playlist_name: &str,
    songs: Vec<SongImport>,
) -> (String, usize) {
    let lib = match active_library_mut(&mut libs.libraries) {
        Some(l) => l,
        None => {
            crate::log_info!("library", "sync_playlist: 没有激活的库");
            return (String::new(), 0);
        }
    };

    // 1. 创建/查找歌单同名文件夹
    let folder_id = if let Some(f) = lib.folders.iter().find(|f| f.name == playlist_name) {
        f.id.clone()
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        lib.folders.push(Folder {
            id: id.clone(),
            name: playlist_name.to_string(),
            parent_id: None,
            song_ids: vec![],
            system: false,
        });
        id
    };

    // 2. 逐首歌入库
    let mut synced = 0usize;
    for song_import in &songs {
        let music_id = make_music_id(&song_import.source, &song_import.source_id);

        // 插入 songs map（有则跳过）
        if !libs.songs.contains_key(&music_id) {
            let entry = SongEntry {
                music_id: music_id.clone(),
                title: song_import.title.clone(),
                artist: song_import.artist.clone(),
                source: song_import.source.clone(),
                source_id: song_import.source_id.clone(),
                file_path: song_import.file_path.clone(),
                file_size: 0,
                lyrics_edited: false,
            };
            libs.songs.insert(music_id.clone(), entry);
        }

        // 放入默认文件夹（不重复）
        if let Some(default_f) = lib.folders.iter_mut().find(|f| f.name == "默认") {
            if !default_f.song_ids.contains(&music_id) {
                default_f.song_ids.push(music_id.clone());
            }
        }

        // 放入歌单文件夹（不重复）
        if let Some(pl_f) = lib.find_folder_mut(&folder_id) {
            if !pl_f.song_ids.contains(&music_id) {
                pl_f.song_ids.push(music_id.clone());
            }
        }

        // P8-3: 自动归类到系统源文件夹
        if let Some(sys_folder) = lib.folders.iter().find(|f| {
            f.system && f.name.to_lowercase().contains(&song_import.source)
        }) {
            let sys_id = sys_folder.id.clone();
            if let Some(sf) = lib.find_folder_mut(&sys_id) {
                if !sf.song_ids.contains(&music_id) {
                    sf.song_ids.push(music_id.clone());
                }
            }
        }

        synced += 1;
    }

    crate::log_info!("library", "sync_playlist: {playlist_name} → {synced} 首");
    (folder_id, synced)
}
