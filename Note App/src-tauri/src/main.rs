// TO BE DELETED
use chrono;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// Struktur untuk serialisasi commit (untuk frontend)
#[derive(Serialize, Deserialize)]
struct CommitInfo {
    id: String,
    date: String,
    files: Vec<String>,
}

// Struktur untuk kategori
#[derive(Serialize, Deserialize, Clone)]
struct Note {
    id: String,
    name: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Category {
    id: String,
    name: String,
    notes: Vec<Note>,
}

// Dapatkan path ke .noteApp di home directory
fn get_noteapp_dir() -> String {
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/Users/ob".to_string());
    format!("{}/.noteApp", home_dir)
}

// Inisialisasi aplikasi dan VCS
#[tauri::command]
fn init_app() -> Result<(), String> {
    let noteapp_dir = get_noteapp_dir();
    let objects_dir = format!("{}/.vcs/objects", noteapp_dir);
    let categories_dir = format!("{}/categories", noteapp_dir);

    // Cek dan buat folder .noteApp kalau belum ada
    if !Path::new(&noteapp_dir).exists() {
        fs::create_dir_all(&noteapp_dir).map_err(|e| e.to_string())?;
    }

    // Inisialisasi VCS
    fs::create_dir_all(&objects_dir).map_err(|e| e.to_string())?;
    
    // Inisialisasi direktori kategori
    fs::create_dir_all(&categories_dir).map_err(|e| e.to_string())?;
    
    // Buat file kategori.json jika belum ada
    let categories_file = format!("{}/categories.json", noteapp_dir);
    if !Path::new(&categories_file).exists() {
        let empty_categories: Vec<Category> = Vec::new();
        let json = serde_json::to_string_pretty(&empty_categories).map_err(|e| e.to_string())?;
        fs::write(&categories_file, json).map_err(|e| e.to_string())?;
    }
    
    println!("Initialized app in {}", noteapp_dir);
    Ok(())
}

// Baca kategori dari file
fn read_categories() -> Result<Vec<Category>, String> {
    let noteapp_dir = get_noteapp_dir();
    let categories_file = format!("{}/categories.json", noteapp_dir);
    
    if !Path::new(&categories_file).exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(&categories_file).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

// Simpan kategori ke file
fn save_categories(categories: &Vec<Category>) -> Result<(), String> {
    let noteapp_dir = get_noteapp_dir();
    let categories_file = format!("{}/categories.json", noteapp_dir);
    
    let json = serde_json::to_string_pretty(categories).map_err(|e| e.to_string())?;
    fs::write(&categories_file, json).map_err(|e| e.to_string())
}

// Dapatkan semua kategori
#[tauri::command]
fn get_categories() -> Result<Vec<Category>, String> {
    read_categories()
}

// Buat kategori baru
#[tauri::command]
fn create_category(name: String) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Category name cannot be empty".to_string());
    }
    
    let mut categories = read_categories()?;
    
    // Cek duplikat nama
    if categories.iter().any(|c| c.name == name) {
        return Err(format!("Category '{}' already exists", name));
    }
    
    let id = Uuid::new_v4().to_string();
    let category = Category {
        id: id.clone(),
        name,
        notes: Vec::new(),
    };
    
    categories.push(category);
    save_categories(&categories)?;
    
    Ok(id)
}

// Rename kategori
#[tauri::command]
fn rename_category(id: String, new_name: String) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("Category name cannot be empty".to_string());
    }
    
    let mut categories = read_categories()?;
    
    // Cek duplikat nama
    if categories.iter().any(|c| c.name == new_name && c.id != id) {
        return Err(format!("Category '{}' already exists", new_name));
    }
    
    if let Some(category) = categories.iter_mut().find(|c| c.id == id) {
        category.name = new_name;
        save_categories(&categories)?;
        Ok(())
    } else {
        Err(format!("Category not found"))
    }
}

// Hapus kategori
#[tauri::command]
fn delete_category(id: String) -> Result<(), String> {
    let mut categories = read_categories()?;
    
    let initial_len = categories.len();
    categories.retain(|c| c.id != id);
    
    if categories.len() < initial_len {
        // Hapus semua file note untuk kategori ini
        let noteapp_dir = get_noteapp_dir();
        let category_dir = format!("{}/categories/{}", noteapp_dir, id);
        
        if Path::new(&category_dir).exists() {
            fs::remove_dir_all(&category_dir).map_err(|e| e.to_string())?;
        }
        
        save_categories(&categories)?;
        Ok(())
    } else {
        Err(format!("Category not found"))
    }
}

// Buat note baru
#[tauri::command]
fn create_note(category_id: String, name: String) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Note name cannot be empty".to_string());
    }
    
    let mut categories = read_categories()?;
    
    let category = match categories.iter_mut().find(|c| c.id == category_id) {
        Some(c) => c,
        None => return Err(format!("Category not found")),
    };
    
    // Cek duplikat nama
    if category.notes.iter().any(|n| n.name == name) {
        return Err(format!("Note '{}' already exists in this category", name));
    }
    
    let id = Uuid::new_v4().to_string();
    let note = Note {
        id: id.clone(),
        name,
    };
    
    // Buat direktori dan file note kosong
    let noteapp_dir = get_noteapp_dir();
    let category_dir = format!("{}/categories/{}", noteapp_dir, category_id);
    fs::create_dir_all(&category_dir).map_err(|e| e.to_string())?;
    
    let note_file = format!("{}/{}.txt", category_dir, id);
    File::create(&note_file).map_err(|e| e.to_string())?;
    
    // Buat direktori untuk history commit
    let commit_dir = format!("{}/categories/{}/{}/.commits", noteapp_dir, category_id, id);
    fs::create_dir_all(&commit_dir).map_err(|e| e.to_string())?;
    
    category.notes.push(note);
    save_categories(&categories)?;
    
    Ok(id)
}

// Rename note
#[tauri::command]
fn rename_note(category_id: String, note_id: String, new_name: String) -> Result<(), String> {
    if new_name.trim().is_empty() {
        return Err("Note name cannot be empty".to_string());
    }
    
    let mut categories = read_categories()?;
    
    let category = match categories.iter_mut().find(|c| c.id == category_id) {
        Some(c) => c,
        None => return Err(format!("Category not found")),
    };
    
    // Cek duplikat nama
    if category.notes.iter().any(|n| n.name == new_name && n.id != note_id) {
        return Err(format!("Note '{}' already exists in this category", new_name));
    }
    
    if let Some(note) = category.notes.iter_mut().find(|n| n.id == note_id) {
        note.name = new_name;
        save_categories(&categories)?;
        Ok(())
    } else {
        Err(format!("Note not found"))
    }
}

// Hapus note
#[tauri::command]
fn delete_note(category_id: String, note_id: String) -> Result<(), String> {
    let mut categories = read_categories()?;
    
    let category = match categories.iter_mut().find(|c| c.id == category_id) {
        Some(c) => c,
        None => return Err(format!("Category not found")),
    };
    
    let initial_len = category.notes.len();
    category.notes.retain(|n| n.id != note_id);
    
    if category.notes.len() < initial_len {
        // Hapus file note
        let noteapp_dir = get_noteapp_dir();
        let note_dir = format!("{}/categories/{}/{}", noteapp_dir, category_id, note_id);
        let note_file = format!("{}/categories/{}/{}.txt", noteapp_dir, category_id, note_id);
        
        if Path::new(&note_file).exists() {
            fs::remove_file(&note_file).map_err(|e| e.to_string())?;
        }
        
        // Hapus direktori commit jika ada
        if Path::new(&note_dir).exists() {
            fs::remove_dir_all(&note_dir).map_err(|e| e.to_string())?;
        }
        
        save_categories(&categories)?;
        Ok(())
    } else {
        Err(format!("Note not found"))
    }
}

// Hitung hash SHA-256 dari content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

// Simpan snapshot file
fn save_content_snapshot(content: &str, hash: &str) -> Result<(), String> {
    let noteapp_dir = get_noteapp_dir();
    let subdir = &hash[0..2];
    let rest = &hash[2..];
    let dir_path = format!("{}/.vcs/objects/{}", noteapp_dir, subdir);
    let file_path = format!("{}/.vcs/objects/{}/{}", noteapp_dir, subdir, rest);

    if !Path::new(&file_path).exists() {
        fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;
        fs::write(&file_path, content).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

// Tambah dan commit note
#[tauri::command]
fn save_and_commit(category_id: String, note_id: String, content: String) -> Result<String, String> {
    // Validasi kategori dan note
    let categories = read_categories()?;
    
    let category = match categories.iter().find(|c| c.id == category_id) {
        Some(c) => c,
        None => return Err(format!("Category not found")),
    };
    
    if !category.notes.iter().any(|n| n.id == note_id) {
        return Err(format!("Note not found"));
    }
    
    // Simpan konten ke file note
    let noteapp_dir = get_noteapp_dir();
    let note_file = format!("{}/categories/{}/{}.txt", noteapp_dir, category_id, note_id);
    fs::write(&note_file, &content).map_err(|e| e.to_string())?;
    
    // Compute hash dan simpan di objects
    let hash = compute_hash(&content);
    save_content_snapshot(&content, &hash)?;
    
    // Buat commit
    let commit_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    
    let commit_dir = format!("{}/categories/{}/{}/.commits", noteapp_dir, category_id, note_id);
    fs::create_dir_all(&commit_dir).map_err(|e| e.to_string())?;
    
    let commit_path = format!("{}/{}", commit_dir, commit_id);
    fs::write(&commit_path, &hash).map_err(|e| e.to_string())?;
    
    Ok(format!("Created commit {}", commit_id))
}

// Baca riwayat commit untuk note tertentu
#[tauri::command]
fn show_history(category_id: String, note_id: String) -> Result<Vec<CommitInfo>, String> {
    let noteapp_dir = get_noteapp_dir();
    let commits_dir = format!("{}/categories/{}/{}/.commits", noteapp_dir, category_id, note_id);
    let mut commits = Vec::new();

    if !Path::new(&commits_dir).exists() {
        return Ok(vec![]);
    }

    let mut commit_ids: Vec<String> = fs::read_dir(&commits_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .collect();
    
    commit_ids.sort_by(|a, b| b.cmp(a)); // Urutkan terbalik agar yang terbaru di atas

    for commit_id in commit_ids {
        let commit_path = format!("{}/{}", commits_dir, commit_id);
        let _content = fs::read_to_string(&commit_path).map_err(|e| e.to_string())?;
        let timestamp = commit_id.parse::<u64>().unwrap();
        let time_str = std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp);
        let date = chrono::DateTime::<chrono::Utc>::from(time_str)
            .with_timezone(&chrono::Local)
            .format("%a %b %d %H:%M:%S %Y")
            .to_string();

        commits.push(CommitInfo {
            id: commit_id,
            date,
            files: vec![format!("{}.txt", note_id)],
        });
    }

    Ok(commits)
}

// Baca konten dari commit tertentu
#[tauri::command]
fn read_commit(category_id: String, note_id: String, commit_id: String) -> Result<String, String> {
    let noteapp_dir = get_noteapp_dir();
    let commit_path = format!("{}/categories/{}/{}/.commits/{}", noteapp_dir, category_id, note_id, commit_id);
    
    // Baca hash dari file commit
    let hash = fs::read_to_string(&commit_path).map_err(|e| e.to_string())?;
    
    // Akses file dari objects
    let subdir = &hash[0..2];
    let rest = &hash[2..];
    let object_path = format!("{}/.vcs/objects/{}/{}", noteapp_dir, subdir, rest);
    
    // Baca isi file dari objects
    let file_content = fs::read_to_string(&object_path).map_err(|e| e.to_string())?;
    
    Ok(file_content)
}

// Baca konten note
#[tauri::command]
fn read_note(category_id: String, note_id: String) -> Result<String, String> {
    let noteapp_dir = get_noteapp_dir();
    let note_path = format!("{}/categories/{}/{}.txt", noteapp_dir, category_id, note_id);

    if !Path::new(&note_path).exists() {
        return Ok("".to_string());
    }

    let content = fs::read_to_string(&note_path).map_err(|e| e.to_string())?;
    Ok(content)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            init_app,
            get_categories,
            create_category,
            rename_category,
            delete_category,
            create_note,
            rename_note,
            delete_note,
            save_and_commit,
            show_history,
            read_commit,
            read_note
        ])        
        .setup(|_app| {  
            init_app()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
