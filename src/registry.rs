//! SQLite review registry — plugin 自有表，與 graphify-registry 共用同一 graphify.db 檔。
//!
//! ## 表
//! - `review_bindings`: (workspace_key, review_id → canonical node, severity,
//!   status, comment…) — CRG Review 點位升維綁定後的持久化。
//!
//! ## 為什麼不用 graphify-registry 的 RegistryDb？
//! `RegistryDb.conn` 為 private，不暴露 raw execute。本 plugin 以獨立
//! `rusqlite::Connection` 開啟同一 db 檔，建立自有表（`CREATE TABLE IF NOT
//! EXISTS`），不干涉 graphify-registry 的 schema 版本管理。
//!
//! ## 與 GEMINI spec 的偏差
//! spec 的 `review_registry.sqlite`（獨立檔案、`Id` 單一 PK）在共享
//! graphify.db 的情境下不成立：多 workspace 會撞 review_id。故 PK 改為
//! `(workspace_key, id)`，與 opendoc_links 同款 workspace scoping。

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

/// 一筆 review 綁定（review_bindings 列）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBinding {
    pub workspace_key: String,
    pub id: String,
    pub canonical_node_id: String,
    pub file_path: String,
    pub line_number: i64,
    /// 綁定時 AST 節點結構 hash（Slice 1 drift guard 用；Slice 0 存空字串）。
    pub signature_hash: String,
    pub severity: String,
    pub category: String,
    pub comment: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ReviewBinding {
    /// 是否未解決。
    #[must_use]
    pub fn is_unresolved(&self) -> bool {
        self.status == "unresolved"
    }
}

/// plugin 自有 SQLite 連線。
pub struct ReviewDb {
    conn: Connection,
}

impl ReviewDb {
    /// 開啟 `path`（共用的 graphify.db），並確保 plugin schema 已建。
    ///
    /// # Errors
    /// 回傳 `rusqlite::Error` 於開啟或 DDL 執行失敗時。
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS review_bindings (
                workspace_key     TEXT NOT NULL,
                id                TEXT NOT NULL,
                canonical_node_id TEXT NOT NULL,
                file_path         TEXT NOT NULL,
                line_number       INTEGER NOT NULL,
                signature_hash    TEXT NOT NULL,
                severity          TEXT NOT NULL,
                category          TEXT NOT NULL,
                comment           TEXT NOT NULL,
                status            TEXT NOT NULL DEFAULT 'unresolved',
                created_at        TEXT NOT NULL,
                updated_at        TEXT NOT NULL,
                PRIMARY KEY (workspace_key, id)
            );

            CREATE INDEX IF NOT EXISTS idx_review_node
                ON review_bindings (workspace_key, canonical_node_id);

            CREATE INDEX IF NOT EXISTS idx_review_status
                ON review_bindings (workspace_key, status);",
        )?;
        Ok(Self { conn })
    }

    /// 插入或覆寫一筆綁定（以 review_id 為鍵）。已存在則更新除
    /// `created_at` 外的所有欄位並刷新 `updated_at`。
    ///
    /// # Errors
    /// SQLite DML 失敗時回傳 `rusqlite::Error`。
    pub fn upsert(&self, b: &ReviewBinding) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO review_bindings
                (workspace_key, id, canonical_node_id, file_path, line_number,
                 signature_hash, severity, category, comment, status,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(workspace_key, id) DO UPDATE SET
                canonical_node_id = excluded.canonical_node_id,
                file_path         = excluded.file_path,
                line_number       = excluded.line_number,
                signature_hash    = excluded.signature_hash,
                severity          = excluded.severity,
                category          = excluded.category,
                comment           = excluded.comment,
                status            = excluded.status,
                updated_at        = excluded.updated_at",
            rusqlite::params![
                b.workspace_key,
                b.id,
                b.canonical_node_id,
                b.file_path,
                b.line_number,
                b.signature_hash,
                b.severity,
                b.category,
                b.comment,
                b.status,
                b.created_at,
                b.updated_at,
            ],
        )?;
        Ok(())
    }

    /// 查詢一個 workspace 中指定 canonical node 的所有綁定。
    ///
    /// # Errors
    /// SQLite 查詢失敗時回傳 `rusqlite::Error`。
    pub fn query_by_node(
        &self,
        workspace_key: &str,
        node_id: &str,
    ) -> Result<Vec<ReviewBinding>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT workspace_key, id, canonical_node_id, file_path, line_number,
                    signature_hash, severity, category, comment, status,
                    created_at, updated_at
             FROM review_bindings
             WHERE workspace_key = ?1 AND canonical_node_id = ?2
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([workspace_key, node_id], row_from_sql)?;
        rows.collect()
    }

    /// 查詢指定 node 未解決的綁定（review_get_context 主路徑）。
    ///
    /// # Errors
    /// SQLite 查詢失敗時回傳 `rusqlite::Error`。
    pub fn query_unresolved_by_node(
        &self,
        workspace_key: &str,
        node_id: &str,
    ) -> Result<Vec<ReviewBinding>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT workspace_key, id, canonical_node_id, file_path, line_number,
                    signature_hash, severity, category, comment, status,
                    created_at, updated_at
             FROM review_bindings
             WHERE workspace_key = ?1 AND canonical_node_id = ?2
               AND status = 'unresolved'
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([workspace_key, node_id], row_from_sql)?;
        rows.collect()
    }

    /// 標記一筆綁定為 resolved（manual / auto）。回傳受影響列數（0 = review_id 不存在）。
    ///
    /// # Errors
    /// SQLite DML 失敗時回傳 `rusqlite::Error`。
    pub fn resolve(
        &self,
        workspace_key: &str,
        review_id: &str,
        updated_at: &str,
    ) -> Result<usize, rusqlite::Error> {
        self.conn.execute(
            "UPDATE review_bindings
             SET status = 'resolved', updated_at = ?3
             WHERE workspace_key = ?1 AND id = ?2",
            rusqlite::params![workspace_key, review_id, updated_at],
        )
    }

    /// 統計一個 workspace 的綁定數。
    ///
    /// # Errors
    /// SQLite 查詢失敗時回傳 `rusqlite::Error`。
    pub fn count(&self, workspace_key: &str) -> Result<usize, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM review_bindings WHERE workspace_key = ?1",
                [workspace_key],
                |r| r.get(0),
            )
            .map(|n: i64| n as usize)
    }

    /// 統計一個 workspace 未解決的綁定數（sync_toon 摘要用）。
    ///
    /// # Errors
    /// SQLite 查詢失敗時回傳 `rusqlite::Error`。
    pub fn count_unresolved(&self, workspace_key: &str) -> Result<usize, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM review_bindings
                 WHERE workspace_key = ?1 AND status = 'unresolved'",
                [workspace_key],
                |r| r.get(0),
            )
            .map(|n: i64| n as usize)
    }

    /// 依 review_id 找一筆綁定（銷案/檢查用）。
    ///
    /// # Errors
    /// SQLite 查詢失敗時回傳 `rusqlite::Error`。
    pub fn get(
        &self,
        workspace_key: &str,
        review_id: &str,
    ) -> Result<Option<ReviewBinding>, rusqlite::Error> {
        self.conn
            .query_row(
                "SELECT workspace_key, id, canonical_node_id, file_path, line_number,
                        signature_hash, severity, category, comment, status,
                        created_at, updated_at
                 FROM review_bindings
                 WHERE workspace_key = ?1 AND id = ?2",
                rusqlite::params![workspace_key, review_id],
                row_from_sql,
            )
            .optional()
    }
}

fn row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReviewBinding> {
    Ok(ReviewBinding {
        workspace_key: row.get(0)?,
        id: row.get(1)?,
        canonical_node_id: row.get(2)?,
        file_path: row.get(3)?,
        line_number: row.get(4)?,
        signature_hash: row.get(5)?,
        severity: row.get(6)?,
        category: row.get(7)?,
        comment: row.get(8)?,
        status: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(ws: &str, id: &str, node: &str) -> ReviewBinding {
        ReviewBinding {
            workspace_key: ws.to_string(),
            id: id.to_string(),
            canonical_node_id: node.to_string(),
            file_path: "src/auth.rs".to_string(),
            line_number: 42,
            signature_hash: String::new(),
            severity: "high".to_string(),
            category: "security".to_string(),
            comment: "timing attack".to_string(),
            status: "unresolved".to_string(),
            created_at: "2026-08-10T00:00:00Z".to_string(),
            updated_at: "2026-08-10T00:00:00Z".to_string(),
        }
    }

    fn open_tmp() -> (tempfile::TempDir, ReviewDb) {
        let dir = tempfile::tempdir().unwrap();
        let db = ReviewDb::open(&dir.path().join("graphify.db")).unwrap();
        (dir, db)
    }

    #[test]
    fn upsert_and_query_by_node() {
        let (_d, db) = open_tmp();
        db.upsert(&binding("w-1", "crg-1", "src/auth.rs:function:verify_token"))
            .unwrap();
        let rows = db
            .query_by_node("w-1", "src/auth.rs:function:verify_token")
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "crg-1");
        assert!(rows[0].is_unresolved());
    }

    #[test]
    fn upsert_same_review_id_replaces() {
        let (_d, db) = open_tmp();
        let mut b = binding("w-1", "crg-1", "src/auth.rs:function:verify_token");
        db.upsert(&b).unwrap();
        b.canonical_node_id = "src/auth.rs:function:login".to_string();
        b.status = "resolved".to_string();
        db.upsert(&b).unwrap();
        let rows = db
            .query_by_node("w-1", "src/auth.rs:function:login")
            .unwrap();
        assert_eq!(rows.len(), 1, "replaced, not duplicated");
        assert_eq!(rows[0].status, "resolved");
        let old = db
            .query_by_node("w-1", "src/auth.rs:function:verify_token")
            .unwrap();
        assert!(old.is_empty());
    }

    #[test]
    fn workspace_isolation() {
        let (_d, db) = open_tmp();
        db.upsert(&binding("w-1", "crg-1", "n")).unwrap();
        db.upsert(&binding("w-2", "crg-1", "n")).unwrap();
        assert_eq!(db.count("w-1").unwrap(), 1);
        assert_eq!(db.count("w-2").unwrap(), 1);
        assert_eq!(db.query_by_node("w-1", "n").unwrap().len(), 1);
    }

    #[test]
    fn unresolved_filter_and_resolve() {
        let (_d, db) = open_tmp();
        let mut b = binding("w-1", "crg-1", "n");
        db.upsert(&b).unwrap();
        b.status = "dismissed".to_string();
        db.upsert(&b).unwrap();
        let un = db.query_unresolved_by_node("w-1", "n").unwrap();
        assert_eq!(un.len(), 0, "dismissed is not unresolved");

        let n = db.resolve("w-1", "crg-1", "2026-08-11T00:00:00Z").unwrap();
        assert_eq!(n, 1);
        let row = db.get("w-1", "crg-1").unwrap().unwrap();
        assert_eq!(row.status, "resolved");
        assert_eq!(row.updated_at, "2026-08-11T00:00:00Z");
    }

    #[test]
    fn resolve_unknown_id_returns_zero() {
        let (_d, db) = open_tmp();
        let n = db.resolve("w-1", "nope", "now").unwrap();
        assert_eq!(n, 0);
    }
}
