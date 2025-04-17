<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
</script>

<template>
  <nav>
    <router-link to="/">Home</router-link>
    <router-link to="/about">About</router-link>
  </nav>
  <main>
    <router-view></router-view>
  </main>
</template>
