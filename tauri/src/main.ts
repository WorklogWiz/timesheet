import { mount } from 'svelte';
import App from './App.svelte';

console.log('main.ts loaded');
console.log('App element:', document.getElementById('app'));

const app = mount(App, {
  target: document.getElementById('app')!,
});

console.log('Svelte app mounted');

export default app;

