// ════════════════════════════════════════════════
// 模块: db
// 路径: src-tauri/src/db.rs
// ────────────────────────────────────────────
// 功能: SQLite 数据库初始化 + 操作
// 标注: 所有 DB 写操作必须返回 Result/Result<String>
// 数据库路径: {LocalAppData}/midou-music/midou-music.db
// 依赖: rusqlite
// ════════════════════════════════════════════════

use rusqlite::Connection;

use crate::types::KugouAuth;

/// 初始化数据库（建表）
///
/// 幂等：CREATE TABLE IF NOT EXISTS
pub fn init_db(db: &Connection) -> Result<(), String> {
    db.execute_batch(
        r#"
        -- 网络收藏
        CREATE TABLE IF NOT EXISTS favorites_net (
            id          INTEGER PRIMARY KEY,
            song_id     TEXT    NOT NULL,
            source      TEXT    NOT NULL,
            song_json   TEXT    NOT NULL,
            added_at    INTEGER NOT NULL
        );

        -- 分类（网络收藏用）
        CREATE TABLE IF NOT EXISTS categories (
            id         INTEGER PRIMARY KEY,
            name       TEXT    NOT NULL,
            type       TEXT    NOT NULL,
            source     TEXT,
            created_at INTEGER NOT NULL
        );

        -- 歌曲 ↔ 分类 多对多
        CREATE TABLE IF NOT EXISTS song_categories (
            song_id     TEXT NOT NULL,
            source      TEXT NOT NULL,
            category_id INTEGER NOT NULL,
            PRIMARY KEY (song_id, source, category_id)
        );

        -- 本地收藏（已下载歌曲）
        CREATE TABLE IF NOT EXISTS favorites_local (
            id         INTEGER PRIMARY KEY,
            file_path  TEXT    NOT NULL UNIQUE,
            title      TEXT,
            artist     TEXT,
            album      TEXT,
            duration   REAL,
            lrc_path   TEXT,
            lrc_offset INTEGER DEFAULT 0,
            added_at   INTEGER NOT NULL
        );

        -- 应用设置（KV）
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- 酷狗登录凭证
        CREATE TABLE IF NOT EXISTS kugou_auth (
            id            INTEGER PRIMARY KEY,
            dfid          TEXT NOT NULL,
            token         TEXT NOT NULL,
            userid        INTEGER NOT NULL,
            vip_token     TEXT NOT NULL,
            vip_type      INTEGER NOT NULL,
            token_expires INTEGER NOT NULL,
            updated_at    INTEGER NOT NULL
        );
        "#,
    )
    .map_err(|e| format!("数据库初始化失败: {e}"))
}

/// 从 DB 加载酷狗凭证（启动时调用）
pub fn load_kugou_auth(db: &Connection) -> KugouAuth {
    let mut stmt = match db.prepare(
        "SELECT dfid,token,userid,vip_token,vip_type,token_expires FROM kugou_auth WHERE id=1",
    ) {
        Ok(s) => s,
        Err(_) => return KugouAuth::default_fallback(),
    };

    let row = match stmt.query_row([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i32>(4)?,
            row.get::<_, i64>(5)?,
        ))
    }) {
        Ok(r) => r,
        Err(_) => return KugouAuth::default_fallback(),
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let logged_in = !row.1.is_empty() && row.5 > now;
    KugouAuth {
        logged_in,
        dfid: row.0,
        token: row.1,
        userid: row.2 as u64,
        vip_token: row.3,
        vip_type: row.4 as u32,
        token_expires: row.5,
    }
}

/// 写入酷狗凭证（登录成功后调用）
pub fn save_kugou_auth(db: &Connection, auth: &KugouAuth) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    db.execute(
        r#"
        INSERT INTO kugou_auth (id, dfid, token, userid, vip_token, vip_type, token_expires, updated_at)
        VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(id) DO UPDATE SET
            dfid=?1, token=?2, userid=?3, vip_token=?4, vip_type=?5, token_expires=?6, updated_at=?7
        "#,
        rusqlite::params![
            auth.dfid,
            auth.token,
            auth.userid as i64,
            auth.vip_token,
            auth.vip_type as i32,
            auth.token_expires,
            now,
        ],
    )
    .map_err(|e| format!("保存酷狗凭证失败: {e}"))?;

    Ok(())
}

/// 删除酷狗凭证（登出时调用）
pub fn delete_kugou_auth(db: &Connection) -> Result<(), String> {
    db.execute("DELETE FROM kugou_auth", [])
        .map_err(|e| format!("删除酷狗凭证失败: {e}"))?;
    Ok(())
}
