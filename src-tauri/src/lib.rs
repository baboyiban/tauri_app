// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn calc(a: &str, b: &str) -> String {
    let result: f64 = a.parse::<f64>().unwrap() + b.parse::<f64>().unwrap();
    format!("{} + {} = {}", a, b, result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![calc])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
