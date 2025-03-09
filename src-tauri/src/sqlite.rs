use rusqlite::{Connection, Result};

#[derive(Debug)]
struct Folder {
    id: i32,
    path: String,
}

pub fn init_db() -> Result<()> {
    let conn = Connection::open("boombox.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE
        )",
        [],
    )?;
    println!("Database initialized");

    let mut stmt = conn.prepare("SELECT * FROM folders")?;
    let folder_iter = stmt.query_map([], |row| {
        Ok(Folder {
            id: row.get(0)?,
            path: row.get(1)?,
        })
    })?;

    println!("Folders in database:");
    for folder in folder_iter {
        println!("{:?}", folder?);
    }

    Ok(())
}

pub fn add_folders(folders: Vec<String>) -> Result<()> {
    let conn = Connection::open("boombox.db")?;
    for folder in folders {
        conn.execute("INSERT INTO folders (path) VALUES (?)", &[&folder])?;
    }
    Ok(())
}

pub fn get_folders() -> Result<Vec<String>> {
    let conn = Connection::open("boombox.db")?;
    let mut stmt = conn.prepare("SELECT path FROM folders")?;
    let folder_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut folders = Vec::new();
    for folder in folder_iter {
        folders.push(folder?);
    }
    Ok(folders)
}
