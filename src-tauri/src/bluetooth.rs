use std::io::{Read, Write};
use tauri::Runtime;

#[tauri::command]
async fn get_bonded_devices() -> Result<Vec<String>, String> {
    // BluetoothDevice 대신 String으로 단순화
    android_bluetooth_serial::get_bonded_devices()
        .map_err(|e| e.to_string())
        .map(|devices| {
            devices
                .into_iter()
                .map(|d| d.get_address().unwrap_or_default())
                .collect()
        })
}

#[tauri::command]
async fn connect_to_device(device_addr: String, uuid: String) -> Result<(), String> {
    let bonded_devices =
        android_bluetooth_serial::get_bonded_devices().map_err(|e| e.to_string())?;

    let target_device = bonded_devices
        .into_iter()
        .find(|d| {
            d.get_address()
                .map(|addr| addr == device_addr)
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("Device not found: {}", device_addr))?;

    let mut socket = target_device
        .build_rfcomm_socket(&uuid, false)
        .map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        if let Err(e) = socket.connect() {
            eprintln!("Connection failed: {}", e);
        } else {
            println!("Successfully connected!");

            // Write trait 사용
            let _ = socket
                .write(b"AT\r\n")
                .map_err(|e| eprintln!("Write error: {}", e));

            let mut buf = [0u8; 128];
            // Read trait 사용
            if let Ok(len) = socket.read(&mut buf) {
                println!("Received: {:?}", &buf[..len]);
            }
        }
    });

    Ok(())
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("bluetooth")
        .invoke_handler(tauri::generate_handler![
            get_bonded_devices,
            connect_to_device
        ])
        .build()
}
