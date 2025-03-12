use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

// Struktur untuk serialisasi commit (opsional untuk frontend)
#[derive(Serialize, Deserialize)]
struct CommitInfo {
    id: String,
    date: String,
    files: Vec<String>,
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

    // Cek dan buat folder .noteApp kalau belum ada
    if !Path::new(&noteapp_dir).exists() {
        fs::create_dir_all(&noteapp_dir).map_err(|e| e.to_string())?;
        // Buat file note.txt kosong
        File::create(format!("{}/note.txt", noteapp_dir)).map_err(|e| e.to_string())?;
    }

    // Inisialisasi VCS
    fs::create_dir_all(&objects_dir).map_err(|e| e.to_string())?;
    println!("Initialized VCS in {}", objects_dir);
    Ok(())
}

// Hitung hash SHA-256 dari file
fn compute_hash(filename: &str) -> Result<String, String> {
    let mut file = File::open(filename).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024];

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

// Simpan snapshot file
fn save_file_snapshot(filename: &str, hash: &str) -> Result<(), String> {
    let noteapp_dir = get_noteapp_dir();
    let subdir = &hash[0..2];
    let rest = &hash[2..];
    let dir_path = format!("{}/.vcs/objects/{}", noteapp_dir, subdir);
    let file_path = format!("{}/.vcs/objects/{}/{}", noteapp_dir, subdir, rest);

    fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;
    fs::copy(filename, &file_path).map_err(|e| e.to_string())?;
    Ok(())
}

// Tambah semua file di .noteApp dan buat commit (dipanggil saat Cmd + S)
#[tauri::command]
fn save_and_commit(content: String) -> Result<String, String> {
    let noteapp_dir = get_noteapp_dir();
    let note_file = format!("{}/note.txt", noteapp_dir);

    // Tulis konten ke note.txt
    fs::write(&note_file, content).map_err(|e| e.to_string())?;

    // Tambah ke VCS dan commit
    let mut index = File::create(format!("{}/.vcs/index", noteapp_dir)).map_err(|e| e.to_string())?;
    let hash = compute_hash(&note_file)?;
    save_file_snapshot(&note_file, &hash)?;
    writeln!(index, "note.txt {}", hash).map_err(|e| e.to_string())?;
    println!("Added note.txt with hash {}", hash);

    let commit_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let commit_path = format!("{}/.vcs/commits/{}", noteapp_dir, commit_id);
    fs::create_dir_all(format!("{}/.vcs/commits", noteapp_dir)).map_err(|e| e.to_string())?;
    fs::copy(format!("{}/.vcs/index", noteapp_dir), &commit_path).map_err(|e| e.to_string())?;
    Ok(format!("Created commit {}", commit_id))
}

// Tampilkan riwayat commit
#[tauri::command]
fn show_history() -> Result<Vec<String>, String> {
    let noteapp_dir = get_noteapp_dir();
    let commits_dir = format!("{}/.vcs/commits", noteapp_dir);
    let mut commits = Vec::new();

    if !Path::new(&commits_dir).exists() {
        return Ok(vec!["No commit history found".to_string()]);
    }

    let mut commit_ids: Vec<String> = fs::read_dir(&commits_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.file_name().into_string().unwrap()))
        .collect();
    commit_ids.sort();

    for commit_id in commit_ids {
        let commit_path = format!("{}/{}", commits_dir, commit_id);
        let content = fs::read_to_string(&commit_path).map_err(|e| e.to_string())?;
        let timestamp = commit_id.parse::<u64>().unwrap();
        let time_str = std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp);
        let date = chrono::DateTime::<chrono::Utc>::from(time_str)
            .format("%a %b %d %H:%M:%S %Y")
            .to_string();

        let mut commit_info = vec![format!("Commit {}", commit_id), format!("Date: {}", date)];
        for line in content.lines() {
            commit_info.push(format!("  {}", line));
        }
        commits.push(commit_info.join("\n"));
    }

    Ok(commits)
}


fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![init_app, save_and_commit, show_history])
        .setup(|_app| {  
            init_app()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}