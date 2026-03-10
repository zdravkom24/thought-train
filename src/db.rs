use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Thought {
    pub id: i64,
    pub text: String,
    pub category: String,
    pub created_at: String,
    pub pinned: bool,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = Self::db_path();
        std::fs::create_dir_all(path.parent().unwrap()).ok();
        let conn = Connection::open(&path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn db_path() -> PathBuf {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("thought-train");
        data_dir.join("thoughts.db")
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS thoughts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'Uncategorized',
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                pinned INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS categories (
                name TEXT PRIMARY KEY
            );
            INSERT OR IGNORE INTO categories (name) VALUES
                ('Work'), ('Personal'), ('Ideas'), ('Tasks'),
                ('Health'), ('Finance'), ('Learning'), ('Misc');
            "
        )?;
        Ok(())
    }

    pub fn add_thought(&self, text: &str, category: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO thoughts (text, category) VALUES (?1, ?2)",
            params![text, category],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_thoughts(&self, category_filter: Option<&str>) -> Result<Vec<Thought>> {
        let mut thoughts = Vec::new();
        match category_filter {
            Some(cat) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, text, category, created_at, pinned FROM thoughts
                     WHERE category = ?1
                     ORDER BY pinned DESC, created_at DESC"
                )?;
                let rows = stmt.query_map(params![cat], Self::row_to_thought)?;
                for row in rows {
                    thoughts.push(row?);
                }
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, text, category, created_at, pinned FROM thoughts
                     ORDER BY pinned DESC, created_at DESC"
                )?;
                let rows = stmt.query_map([], Self::row_to_thought)?;
                for row in rows {
                    thoughts.push(row?);
                }
            }
        }
        Ok(thoughts)
    }

    fn row_to_thought(row: &rusqlite::Row) -> Result<Thought> {
        Ok(Thought {
            id: row.get(0)?,
            text: row.get(1)?,
            category: row.get(2)?,
            created_at: row.get(3)?,
            pinned: row.get::<_, i32>(4)? != 0,
        })
    }

    pub fn delete_thought(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM thoughts WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn toggle_pin(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE thoughts SET pinned = CASE WHEN pinned = 0 THEN 1 ELSE 0 END WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn update_category(&self, id: i64, category: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE thoughts SET category = ?1 WHERE id = ?2",
            params![category, id],
        )?;
        Ok(())
    }

    pub fn get_categories(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM categories ORDER BY name")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut cats = Vec::new();
        for row in rows {
            cats.push(row?);
        }
        Ok(cats)
    }

    pub fn add_category(&self, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO categories (name) VALUES (?1)",
            params![name],
        )?;
        Ok(())
    }

    pub fn delete_category(&self, name: &str) -> Result<()> {
        // Move thoughts in this category to Misc
        self.conn.execute(
            "UPDATE thoughts SET category = 'Misc' WHERE category = ?1",
            params![name],
        )?;
        self.conn.execute(
            "DELETE FROM categories WHERE name = ?1",
            params![name],
        )?;
        Ok(())
    }

    pub fn update_thought_text(&self, id: i64, text: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE thoughts SET text = ?1 WHERE id = ?2",
            params![text, id],
        )?;
        Ok(())
    }

    pub fn search_thoughts(&self, query: &str) -> Result<Vec<Thought>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, text, category, created_at, pinned FROM thoughts
             WHERE text LIKE ?1
             ORDER BY pinned DESC, created_at DESC"
        )?;
        let rows = stmt.query_map(params![pattern], Self::row_to_thought)?;
        let mut thoughts = Vec::new();
        for row in rows {
            thoughts.push(row?);
        }
        Ok(thoughts)
    }
}
