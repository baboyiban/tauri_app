<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const a = ref(0);
const b = ref(0);
const result = ref("");

async function calc() {
    result.value = await invoke("calc", { a: a.value, b: b.value });
}
</script>

<template>
    <main>
        <input v-model="a" type="string" />
        <input v-model="b" type="string" />
        <button @click="calc">calc</button>
        <div>{{ result }}</div>
    </main>
</template>

<style scoped>
main {
    background-color: #f0f0f0;
}
</style>
<style></style>
