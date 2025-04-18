<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Bluetooth from "./components/BluetoothView.vue";
import "./assets/main.css";
</script>

<template>
    <main>
        <Bluetooth />
    </main>
</template>

<style scoped>
</style>
<style></style>
