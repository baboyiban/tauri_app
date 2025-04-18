<template>
  <div class="bluetooth-container">
    <h2>Bluetooth Devices</h2>

    <button @click="scanDevices" :disabled="isScanning">
      {{ isScanning ? "Scanning..." : "Scan Devices" }}
    </button>

    <div v-if="error" class="error-message">{{ error }}</div>

    <div class="device-list">
      <div
        v-for="device in devices"
        :key="device.address"
        class="device-item"
        @click="connectDevice(device)"
        :class="{ connected: connectedDevice?.address === device.address }"
      >
        <span class="device-name">{{ device.name || "Unknown" }}</span>
        <span class="device-address">{{ device.address }}</span>
        <span
          v-if="connectedDevice?.address === device.address"
          class="connection-status"
          >✔ Connected</span
        >
      </div>
    </div>

    <div v-if="connectedDevice" class="connection-panel">
      <h3>Connection: {{ connectedDevice.name }}</h3>
      <textarea
        v-model="message"
        placeholder="Enter message to send"
      ></textarea>
      <button @click="sendMessage">Send</button>
      <div class="received-data">
        <h4>Received Data:</h4>
        <pre>{{ receivedData }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

const devices = ref([]);
const isScanning = ref(false);
const error = ref(null);
const connectedDevice = ref(null);
const message = ref("");
const receivedData = ref("");

// 블루투스 데이터 이벤트 리스너 등록
onMounted(async () => {
  await listen("bluetooth-data", (event) => {
    receivedData.value += `${event.payload}\n`;
  });
});

const scanDevices = async () => {
  isScanning.value = true;
  error.value = null;
  try {
    devices.value = await invoke("get_bonded_devices");
  } catch (err) {
    error.value = `Failed to scan devices: ${err}`;
  } finally {
    isScanning.value = false;
  }
};

const connectDevice = async (device) => {
  try {
    await invoke("connect_to_device", {
      deviceAddr: device.address,
      uuid: "00001101-0000-1000-8000-00805F9B34FB", // SPP UUID
    });
    connectedDevice.value = device;
    error.value = null;
  } catch (err) {
    error.value = `Connection failed: ${err}`;
    connectedDevice.value = null;
  }
};

const sendMessage = async () => {
  if (!message.value.trim() || !connectedDevice.value) return;

  try {
    await invoke("send_bluetooth_message", {
      message: message.value,
    });
    message.value = "";
  } catch (err) {
    error.value = `Failed to send message: ${err}`;
  }
};
</script>

<style scoped>
.bluetooth-container {
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
}

.device-list {
  margin-top: 20px;
}

.device-item {
  padding: 10px;
  border: 1px solid #ddd;
  margin-bottom: 8px;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  justify-content: space-between;
}

.device-item:hover {
  background-color: #f5f5f5;
}

.device-item.connected {
  border-color: #4caf50;
  background-color: #e8f5e9;
}

.device-name {
  font-weight: bold;
}

.device-address {
  color: #666;
  font-family: monospace;
}

.connection-status {
  color: #4caf50;
}

.error-message {
  color: #f44336;
  margin: 10px 0;
}

.connection-panel {
  margin-top: 20px;
  padding: 15px;
  border: 1px solid #eee;
  border-radius: 4px;
}

textarea {
  width: 100%;
  min-height: 80px;
  margin: 10px 0;
  padding: 8px;
}

button {
  background-color: #2196f3;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 4px;
  cursor: pointer;
}

button:disabled {
  background-color: #cccccc;
}

.received-data {
  margin-top: 15px;
  padding: 10px;
  background-color: #f9f9f9;
  border-radius: 4px;
}
</style>
