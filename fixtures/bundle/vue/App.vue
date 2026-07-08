<template>
  <div class="card">
    <h1 v-if="title">{{ title }}</h1>
    <ul>
      <li v-for="item in items" :key="item.id">{{ item.label }}</li>
    </ul>
    <button @click="onClick">go</button>
  </div>
</template>
