import './styles/app.css';
import App from './App.svelte';
import { mount } from 'svelte';

const target = document.getElementById('app');

if (!target) {
  throw new Error('Application root was not found');
}

const app = mount(App, { target });

export default app;
