import { createApp } from "vue";
import App from "./App.vue";
import { createPinia } from 'pinia';
import './style.css';
import './desktop.css';

createApp(App).use(createPinia()).mount("#app");
