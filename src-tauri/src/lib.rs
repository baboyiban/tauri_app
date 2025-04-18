use android_bluetooth_serial::{self, BluetoothDevice, BluetoothSocket};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::{Read, Write}, sync::Arc, thread};
use tauri::{Manager, State}; // Tauri 상태 관리 및 이벤트 발행을 위해 필요

// 프런트엔드로 보낼 장치 정보를 담을 구조체 (Serialize 가능해야 함)
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Device {
    address: String,
    name: String,
}

// 프런트엔드로 보낼 수신 데이터 및 상태 변경 정보를 담을 구조체
#[derive(Clone, Serialize)]
struct Payload {
    address: String, // 어떤 장치로부터의 데이터인지 식별
    data: Vec<u8>,
}

#[derive(Clone, Serialize)]
struct StatusPayload {
    address: String,
    status: String, // "connected", "disconnected", "error: ..." 등
    error: Option<String>,
}


// Rust 백엔드에서 BluetoothSocket 인스턴스를 관리할 상태 구조체
// HashMap: 장치 주소를 키로, BluetoothSocket (스레드 안전하게 공유)을 값으로 저장
struct AppState {
    connections: RwLock<HashMap<String, Arc<Mutex<BluetoothSocket>>>>,
}

// AppState 초기화
impl Default for AppState {
    fn default() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }
}

// Tauri Command 정의 시작

// Bluetooth 활성화 상태 확인
#[tauri::command]
async fn is_bluetooth_enabled_command() -> Result<bool, String> {
    android_bluetooth_serial::is_enabled().map_err(|e| e.to_string())
}

// 페어링된 장치 목록 가져오기
#[tauri::command]
async fn get_bonded_devices_command() -> Result<Vec<Device>, String> {
    android_bluetooth_serial::get_bonded_devices()
        .map_err(|e| e.to_string())
        .and_then(|devices| {
            devices.into_iter()
                   .map(|dev| {
                       // 장치 주소와 이름 가져오기 (권한 필요 가능)
                       let address = dev.get_address().map_err(|e| e.to_string())?;
                       let name = dev.get_name().unwrap_or_else(|_| "Unknown Device".to_string()); // 이름 가져오기 실패 시 대체

                       Ok(Device { address, name })
                   })
                   .collect::<Result<Vec<Device>, String>>()
        })
}

// 특정 장치에 연결 시도
// address: 연결할 장치의 MAC 주소
// app_handle: 이벤트를 프런트엔드로 보내기 위해 필요
// state: 소켓 인스턴스를 저장하고 관리하기 위해 필요
#[tauri::command]
async fn connect_device_command(address: String, app_handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    println!("Attempting to connect to: {}", address);

    // 이미 연결된 상태인지 확인
    {
        let connections_lock = state.connections.read();
        if connections_lock.contains_key(&address) {
            println!("Already connected to {}", address);
            // 이미 연결 상태임을 프런트엔드에 알림
            let _ = app_handle.emit_all("bluetooth-status", StatusPayload {
                address: address.clone(),
                status: "already_connected".to_string(),
                error: None,
            });
            return Ok(());
        }
    } // Lock released here

    // 페어링된 장치 목록에서 해당 장치 찾기
    let bonded_devices = android_bluetooth_serial::get_bonded_devices().map_err(|e| e.to_string())?;
    let device = bonded_devices.into_iter().find(|d| d.get_address().unwrap_or_default() == address)
        .ok_or_else(|| "Device not found among bonded devices".to_string())?;

    // RFCOMM 소켓 빌드 (SPP UUID 사용, 보안 연결)
    let socket = device.build_rfcomm_socket(android_bluetooth_serial::SPP_UUID, true).map_err(|e| e.to_string())?;
    let arc_socket = Arc::new(Mutex::new(socket));

    // 소켓 연결 시도
    println!("Calling socket.connect() for {}", address);
    arc_socket.lock().connect().map_err(|e| {
        eprintln!("Connection error for {}: {}", address, e);
        // 연결 실패 시 상태 변경 이벤트 발행
        let _ = app_handle.emit_all("bluetooth-status", StatusPayload {
            address: address.clone(),
            status: "connection_failed".to_string(),
            error: Some(e.to_string()),
        });
        e.to_string() // 오류 반환
    })?;

    println!("Connected successfully to: {}", address);

    // 연결 성공 시 상태에 소켓 저장
    state.connections.write().insert(address.clone(), Arc::clone(&arc_socket));

    // 연결 성공 상태 이벤트 발행
    let _ = app_handle.emit_all("bluetooth-status", StatusPayload {
        address: address.clone(),
        status: "connected".to_string(),
        error: None,
    });

    // 백그라운드 읽기 스레드 시작
    let read_socket_arc = Arc::clone(&arc_socket);
    let address_clone = address.clone();
    let app_handle_clone = app_handle.clone(); // 스레드로 전달할 AppHandle 클론
    let state_arc = state.inner().clone(); // 스레드로 전달할 AppState Arc 클론

    thread::spawn(move || {
        println!("Read thread started for {}", address_clone);
        let mut read_buf = vec![0u8; 1024]; // 읽기 버퍼
        loop {
            let mut socket = read_socket_arc.lock(); // 소켓에 락 획득
            match socket.read(&mut read_buf) {
                Ok(len) if len > 0 => {
                    // 데이터 읽기 성공
                    let data = read_buf[..len].to_vec();
                    // 데이터를 프런트엔드로 이벤트 발생
                    let _ = app_handle_clone.emit_all("bluetooth-data", Payload {
                        address: address_clone.clone(),
                        data: data,
                    });
                    // println!("Read {} bytes from {}", len, address_clone); // 디버그 출력
                }
                Ok(_) => {
                    // 0 바이트 읽음 - 연결 끊김 가능성 또는 데이터 없음
                    // 짧게 대기 후 연결 상태 다시 확인
                    drop(socket); // 락 해제
                    thread::sleep(std::time::Duration::from_millis(50));
                    if !read_socket_arc.lock().is_connected().unwrap_or(false) {
                        eprintln!("Read thread: Device {} disconnected.", address_clone);
                        break; // 루프 종료 (스레드 종료)
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    // 타임아웃 - 데이터가 없거나 읽을 준비가 안 됨. 예상된 상황일 수 있음.
                    // 연결 상태 확인 후 계속 루프
                    drop(socket); // 락 해제
                     if !read_socket_arc.lock().is_connected().unwrap_or(false) {
                        eprintln!("Read thread (timeout check): Device {} disconnected.", address_clone);
                        break; // 루프 종료 (스레드 종료)
                    }
                }
                Err(e) => {
                    // 다른 읽기 오류 발생 (연결 끊김 포함)
                    eprintln!("Read thread error for device {}: {}", address_clone, e);
                    // 오류 상태 이벤트 발행
                    let _ = app_handle_clone.emit_all("bluetooth-status", StatusPayload {
                        address: address_clone.clone(),
                        status: "error".to_string(),
                        error: Some(e.to_string()),
                    });
                    break; // 루프 종료 (스레드 종료)
                }
            }
            // 중요: 읽기/쓰기 작업 후에는 반드시 소켓 락을 해제하여 다른 명령이 접근할 수 있도록 해야 함.
            // Ok(len) > 0 케이스에서는 drop(socket)이 없으므로 루프 시작 시 다시 락을 얻음.
            // 타임아웃이나 0바이트 읽기 케이스는 위에서 drop(socket)을 호출함.
        }

        // 읽기 스레드 종료 전, 상태 관리 맵에서 해당 연결 제거 및 연결 끊김 상태 이벤트 발행
        println!("Read thread finished for {}. Cleaning up state.", address_clone);
        state_arc.connections.write().remove(&address_clone); // 상태 맵에서 제거

        // 스레드가 종료되었으므로 연결 끊김 상태 알림 (오류로 종료된 경우 위에서 이미 보냈을 수 있음)
        let _ = app_handle_clone.emit_all("bluetooth-status", StatusPayload {
            address: address_clone.clone(),
            status: "disconnected".to_string(),
            error: None, // 또는 마지막 오류 정보
        });
    });


    Ok(())
}

// 특정 장치에 데이터 전송
// address: 데이터를 전송할 장치의 MAC 주소 (HashMap 키)
// data: 전송할 데이터 바이트 벡터
// state: 소켓 인스턴스 접근을 위해 필요
#[tauri::command]
async fn send_data_command(address: String, data: Vec<u8>, state: State<'_, AppState>) -> Result<(), String> {
    let connections_lock = state.connections.read();
    // 해당 장치의 소켓 인스턴스 찾기
    let socket_arc = connections_lock.get(&address)
        .ok_or_else(|| "Device not connected".to_string())?; // 연결되어 있지 않으면 오류 반환

    let mut socket = socket_arc.lock(); // 소켓에 락 획득

    // 데이터 쓰기
    socket.write_all(&data).map_err(|e| {
        eprintln!("Write error for {}: {}", address, e);
        e.to_string() // 오류 반환
    })?;

    // 버퍼 비우기 (즉시 전송)
    socket.flush().map_err(|e| {
         eprintln!("Flush error for {}: {}", address, e);
        e.to_string()
    })?;

    println!("Sent {} bytes to {}", data.len(), address);
    Ok(())
}

// 특정 장치 연결 해제
// address: 연결 해제할 장치의 MAC 주소 (HashMap 키)
// state: 소켓 인스턴스 접근 및 제거를 위해 필요
#[tauri::command]
async fn disconnect_device_command(address: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut connections_lock = state.connections.write(); // 제거를 위해 쓰기 락 필요

    // 해당 장치의 소켓 인스턴스 찾아서 제거
    if let Some(socket_arc) = connections_lock.remove(&address) {
        let mut socket = socket_arc.lock(); // 소켓에 락 획득

        // 소켓 닫기
        socket.close().map_err(|e| {
             eprintln!("Close error for {}: {}", address, e);
            e.to_string()
        })?;
        println!("Disconnected successfully from: {}", address);
        // 연결 해제 이벤트는 보통 읽기 스레드 종료 시 발생하지만, 여기서 명시적으로 보낼 수도 있음.
        Ok(())
    } else {
        Err("Device not connected".to_string()) // 연결되어 있지 않으면 오류 반환
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 상태 관리 등록
        .manage(AppState::default())
        // Command 핸들러 등록
        .invoke_handler(tauri::generate_handler![
            is_bluetooth_enabled_command,
            get_bonded_devices_command,
            connect_device_command,
            send_data_command,
            disconnect_device_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

