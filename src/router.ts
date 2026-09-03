import { createMemoryHistory, createRouter } from 'vue-router'

import MidiView from './Midi.vue'


const routes = [
  { path: '/', component: MidiView }
]

const router = createRouter({
  history: createMemoryHistory(),
  routes,
})

export default router