<script setup>
import { ref, onMounted, onUnmounted } from 'vue'; // Vue 3 Composition API
import { invoke } from "@tauri-apps/api/core"; // Tauri Command 호출
import { listen } from "@tauri-apps/api/event"; // Tauri Event 수신

// --- 반응형 상태 변수 정의 ---
const bluetoothStatus = ref("Bluetooth Status: Checking...");
const connectionStatus = ref("Connection Status: Idle");
const bondedDevices = ref([]); // 페어링된 장치 목록 [{ address: string, name: string }]
const selectedDevice = ref(null); // 선택된 장치 { address: string, name: string } 또는 null
const isConnected = ref(false); // 현재 연결 상태
const sendDataInput = ref(""); // 보낼 데이터 입력 필드
const dataReceived = ref(""); // 수신된 데이터

// Event 리스너 정리 함수들을 저장할 변수
let unlistenData = null;
let unlistenStatus = null;

// --- Command 호출 함수 (Rust 백엔드로 요청 보내기) ---

async function checkBluetoothStatus() {
    try {
        bluetoothStatus.value = "Checking status...";
        const isEnabled = await invoke("is_bluetooth_enabled_command");
        bluetoothStatus.value = `Bluetooth Enabled: ${isEnabled}`;
        return isEnabled;
    } catch (error) {
        console.error("Error checking bluetooth status:", error);
        bluetoothStatus.value = `Error checking status: ${error}`;
        return false;
    }
}

async function getBondedDevices() {
    try {
        connectionStatus.value = "Fetching bonded devices...";
        // Rust 백엔드의 get_bonded_devices_command 호출
        const devices = await invoke("get_bonded_devices_command");
        console.log("Bonded Devices:", devices);
        bondedDevices.value = devices; // 상태 업데이트 -> Vue가 목록 UI 자동 업데이트
        connectionStatus.value = `Found ${devices.length} bonded devices.`;
    } catch (error) {
        console.error("Error getting bonded devices:", error);
        connectionStatus.value = `Error getting devices: ${error}`;
    }
}

async function connectDevice() {
    if (!selectedDevice.value) {
        alert("Please select a device first.");
        return;
    }
    try {
        connectionStatus.value = `Connecting to ${selectedDevice.value.address}...`;
        // Rust 백엔드의 connect_device_command 호출
        // 연결 결과는 "bluetooth-status" 이벤트로 수신됨.
        await invoke("connect_device_command", { address: selectedDevice.value.address });
        // UI 상태는 이벤트 리스너에서 업데이트됩니다.
    } catch (error) {
        console.error("Error connecting:", error);
        // invoke 자체에서 발생한 오류 (예: Rust 패닉) 처리
        connectionStatus.value = `Connection Invoke Error: ${error}`;
        // 연결 실패 시 isConnected 상태를 false로 설정 (이벤트 리스너에서도 처리하지만, 혹시 모를 경우 대비)
        isConnected.value = false;
    }
}

async function sendData() {
    if (!selectedDevice.value || !isConnected.value) { // 연결 상태인지 확인
        alert("Please connect to a device first.");
        return;
    }
    const data = sendDataInput.value;
    if (!data) return; // 입력값이 없으면 전송 안 함

    try {
        connectionStatus.value = `Sending data to ${selectedDevice.value.address}...`;
        // 문자열을 바이트 배열로 변환
        const textEncoder = new TextEncoder();
        const dataBytes = Array.from(textEncoder.encode(data));

        // Rust 백엔드의 send_data_command 호출
        await invoke("send_data_command", { address: selectedDevice.value.address, data: dataBytes });
        connectionStatus.value = `Data sent to ${selectedDevice.value.address}.`;
        sendDataInput.value = ""; // 입력창 비우기
    } catch (error) {
        console.error("Error sending data:", error);
        connectionStatus.value = `Send Error: ${error}`;
    }
}

async function disconnectDevice() {
    if (!selectedDevice.value || !isConnected.value) { // 연결 상태인지 확인
        alert("No device connected to disconnect.");
        return;
    }
    try {
        connectionStatus.value = `Disconnecting from ${selectedDevice.value.address}...`;
        // Rust 백엔드의 disconnect_device_command 호출
        await invoke("disconnect_device_command", { address: selectedDevice.value.address });
        // UI 상태는 이벤트 리스너에서 업데이트됩니다.
    } catch (error) {
        console.error("Error disconnecting:", error);
        connectionStatus.value = `Disconnect Invoke Error: ${error}`;
        // 오류 발생 시에도 연결 상태를 false로 설정
        isConnected.value = false;
    }
}

// --- UI 상호작용 함수 ---

// 장치 목록에서 장치 선택
function selectDevice(device) {
    selectedDevice.value = device; // 선택된 장치 상태 업데이트
    console.log("Selected device:", device);
    connectionStatus.value = `Selected device: ${device.name} (${device.address})`;

    // 선택 시 연결 상태가 아니면 연결 버튼 활성화
    if (!isConnected.value) {
        // isConnected 상태는 이벤트 리스너에서 업데이트
    }
}

// --- Vue Lifecycle Hooks ---

onMounted(async () => {
    // 컴포넌트 마운트 시 Event 리스너 설정
    unlistenData = await listen("bluetooth-data", (event) => {
        console.log("Received data event:", event.payload);
        const { address, data } = event.payload;
        // Vec<u8> 형태로 받은 데이터를 문자열로 디코딩 (데이터 형식에 따라 다름)
        const textDecoder = new TextDecoder();
        const receivedText = textDecoder.decode(new Uint8Array(data));
        dataReceived.value += `\n[${address}] ${receivedText}`;
        dataReceived.value.scrollTop = dataReceived.value.scrollHeight; // 스크롤을 맨 아래로 이동
    });

    unlistenStatus = await listen("bluetooth-status", (event) => {
        console.log("Status update event:", event.payload);
        const { address, status, error } = event.payload;

        let statusMessage = `Status (${address || 'App'}): ${status}`;
        if (error) {
            statusMessage += ` - Error: ${error}`;
        }
        connectionStatus.value = statusMessage;

        // 연결 상태 변수 업데이트 및 버튼 상태 제어
        if (status === "connected" && selectedDevice.value && address === selectedDevice.value.address) {
            isConnected.value = true;
        } else if (status === "disconnected" && selectedDevice.value && address === selectedDevice.value.address) {
             isConnected.value = false;
             // 선택된 장치가 연결 해제되면 선택 상태 해제 (옵션)
             // selectedDevice.value = null;
        } else if (status === "connection_failed" && selectedDevice.value && address === selectedDevice.value.address) {
             isConnected.value = false;
        }
        // "already_connected" 등 다른 상태 처리 추가 가능

    });


    // 앱 시작 시 Bluetooth 상태 확인
    const isEnabled = await checkBluetoothStatus();
    if (isEnabled) {
        // Bluetooth 활성화 상태이면 페어링된 장치 목록 가져오기
        getBondedDevices();
    } else {
        // Bluetooth 비활성화 상태 - 사용자에게 활성화를 안내해야 함
        connectionStatus.value = "Bluetooth is not enabled. Please enable it in Android settings.";
        // TODO: 사용자에게 Android 설정으로 이동하도록 안내하는 UI/로직 추가 필요
    }
});

onUnmounted(() => {
    // 컴포넌트 언마운트 시 Event 리스너 정리
    if (unlistenData) unlistenData();
    if (unlistenStatus) unlistenStatus();

    // TODO: 앱 종료 시 연결된 소켓들을 모두 끊어주는 로직 추가 필요
    // Rust 백엔에 "disconnect_all" 같은 Command를 만들거나,
    // Tauri의 앱 종료 이벤트(handle_app_exit)를 활용할 수 있음
});
</script>

<template>
  <div class="container">
    <h1>Tauri Bluetooth Serial</h1>

    <p>{{ bluetoothStatus }}</p>
    <p>{{ connectionStatus }}</p>
    <p v-if="selectedDevice">Selected Device: {{ selectedDevice.name }} ({{ selectedDevice.address }})</p>
     <p v-else>Selected Device: None</p>

    <h2>Bonded Devices</h2>
    <ul id="bonded-devices-list">
      <li v-if="bondedDevices.length === 0 && connectionStatus.indexOf('Fetching') === -1">No bonded devices found. Pair devices in Android settings.</li>
      <li v-for="device in bondedDevices"
          :key="device.address"
          @click="selectDevice(device)"
          :class="{ 'selected-device-item': selectedDevice && selectedDevice.address === device.address }"
          class="device-item">
        {{ device.name }} ({{ device.address }})
      </li>
    </ul>

    <div class="controls">
        <button @click="connectDevice" :disabled="!selectedDevice || isConnected">Connect Selected</button>
        <button @click="disconnectDevice" :disabled="!isConnected">Disconnect</button>
    </div>

    <div class="controls">
        <h2>Send Data</h2>
        <input type="text" v-model="sendDataInput" placeholder="Enter data to send" :disabled="!isConnected"/>
        <button @click="sendData" :disabled="!isConnected">Send</button>
    </div>

    <h2>Received Data</h2>
    <textarea v-model="dataReceived" readonly :disabled="!isConnected"></textarea>
  </div>
</template>

<style scoped>
/* 여기에 Vue 컴포넌트 전용 스타일 추가 */
.container { padding: 10px; }
.device-item { padding: 5px; border-bottom: 1px solid #eee; cursor: pointer; }
.device-item:hover { background-color: #f0f0f0; }
.selected-device-item { background-color: #cce5ff; font-weight: bold; } /* 선택된 장치 스타일 */

.controls { margin-top: 10px; }
.controls button { margin-right: 5px; }

#send-data-input { width: 200px; } /* 입력 필드 너비 조정 */
textarea { width: 100%; height: 150px; margin-top: 10px; }
</style>