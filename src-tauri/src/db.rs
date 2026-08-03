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
        "#,
    )
    .map_err(|e| format!("数据库初始化失败: {e}"))
}
