<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import logo from './assets/logo/logo.svg';

const activeNotes = ref(new Set<number>());
const errorMessage = ref('');
const selectedWaveform = ref(8);
const octaveOffset = ref(0);
const pointerDown = ref(false);
const waveformSamples = ref<number[]>([]);
const audioOutputSamples = ref<number[]>([]);
let outputTimer: number | undefined;

type PianoKey = {
  note: number;
  label: string;
  accidental: boolean;
  position?: string;
};

const pianoKeys: PianoKey[] = [
  { note: 48, label: 'C3', accidental: false },
  { note: 49, label: 'C#3', accidental: true, position: 'calc(7.142% - 1.25rem)' },
  { note: 50, label: 'D3', accidental: false },
  { note: 51, label: 'D#3', accidental: true, position: 'calc(14.285% - 1.25rem)' },
  { note: 52, label: 'E3', accidental: false },
  { note: 53, label: 'F3', accidental: false },
  { note: 54, label: 'F#3', accidental: true, position: 'calc(28.571% - 1.25rem)' },
  { note: 55, label: 'G3', accidental: false },
  { note: 56, label: 'G#3', accidental: true, position: 'calc(35.714% - 1.25rem)' },
  { note: 57, label: 'A3', accidental: false },
  { note: 58, label: 'A#3', accidental: true, position: 'calc(42.857% - 1.25rem)' },
  { note: 59, label: 'B3', accidental: false },
  { note: 60, label: 'C4', accidental: false },
  { note: 61, label: 'C#4', accidental: true, position: 'calc(57.142% - 1.25rem)' },
  { note: 62, label: 'D4', accidental: false },
  { note: 63, label: 'D#4', accidental: true, position: 'calc(64.285% - 1.25rem)' },
  { note: 64, label: 'E4', accidental: false },
  { note: 65, label: 'F4', accidental: false },
  { note: 66, label: 'F#4', accidental: true, position: 'calc(78.571% - 1.25rem)' },
  { note: 67, label: 'G4', accidental: false },
  { note: 68, label: 'G#4', accidental: true, position: 'calc(85.714% - 1.25rem)' },
  { note: 69, label: 'A4', accidental: false },
  { note: 70, label: 'A#4', accidental: true, position: 'calc(92.857% - 1.25rem)' },
  { note: 71, label: 'B4', accidental: false },
];

const displayedPianoKeys = computed(() =>
  pianoKeys.map((key) => ({
    ...key,
    note: key.note + octaveOffset.value * 12,
    label: key.label.replace(/\d+$/, (octave) => String(Number(octave) + octaveOffset.value)),
  })),
);
const whiteKeys = computed(() => displayedPianoKeys.value.filter((key) => !key.accidental));
const blackKeys = computed(() => displayedPianoKeys.value.filter((key) => key.accidental));

const waveformNames = [
  '00_AKWF_sin.wav',
  '01_AKWF_saw.wav',
  '02_AKWF_squ.wav',
  '03_AKWF_tri.wav',
  '04_AKWF_ebass_0001.wav',
  '05_AKWF_cello_0001.wav',
  '06_AKWF_violin_0001.wav',
  '07_AKWF_eorgan_0001.wav',
  '08_AKWF_epiano_0001.wav',
  '09_AKWF_overtone_0001.wav',
  'AKWF_granular_0001.wav',
  'AKWF_granular_0002.wav',
  'AKWF_granular_0003.wav',
  'AKWF_granular_0004.wav',
  'AKWF_granular_0005.wav',
  'AKWF_granular_0006.wav',
  'AKWF_granular_0007.wav',
  'AKWF_granular_0008.wav',
  'AKWF_granular_0009.wav',
  'AKWF_granular_0010.wav',
  'AKWF_granular_0011.wav',
  'AKWF_granular_0012.wav',
  'AKWF_granular_0013.wav',
  'AKWF_granular_0014.wav',
  'AKWF_granular_0015.wav',
  'AKWF_granular_0016.wav',
  'AKWF_granular_0017.wav',
  'AKWF_granular_0018.wav',
  'AKWF_granular_0019.wav',
  'AKWF_granular_0020.wav',
  'AKWF_granular_0021.wav',
  'AKWF_granular_0022.wav',
  'AKWF_granular_0023.wav',
  'AKWF_granular_0024.wav',
  'AKWF_granular_0025.wav',
  'AKWF_granular_0026.wav',
  'AKWF_granular_0027.wav',
  'AKWF_granular_0028.wav',
  'AKWF_granular_0029.wav',
  'AKWF_granular_0030.wav',
  'AKWF_granular_0031.wav',
  'AKWF_granular_0032.wav',
  'AKWF_granular_0033.wav',
  'AKWF_granular_0034.wav',
  'AKWF_granular_0035.wav',
  'AKWF_granular_0036.wav',
  'AKWF_granular_0037.wav',
  'AKWF_granular_0038.wav',
  'AKWF_granular_0039.wav',
  'AKWF_granular_0040.wav',
  'AKWF_granular_0041.wav',
  'AKWF_granular_0042.wav',
  'AKWF_granular_0043.wav',
  'AKWF_granular_0044.wav',
  'CONTRA_BASS.wav',
  'HST_SLAP_BASS.wav',
];

const waveforms = waveformNames.map((label, value) => ({ label, value }));

const activeVoiceCount = computed(() => activeNotes.value.size);
const selectedWaveformLabel = computed(
  () => waveformNames[selectedWaveform.value] ?? waveformNames[0],
);

async function startNote(note: number) {
  if (activeNotes.value.has(note)) return;

  errorMessage.value = '';
  activeNotes.value = new Set(activeNotes.value).add(note);
  try {
    await invoke('play_note', { note, velocity: 100 });
  } catch (error) {
    const notes = new Set(activeNotes.value);
    notes.delete(note);
    activeNotes.value = notes;
    errorMessage.value = String(error);
  }
}

async function stopNote(note: number) {
  if (!activeNotes.value.has(note)) return;

  const notes = new Set(activeNotes.value);
  notes.delete(note);
  activeNotes.value = notes;
  try {
    await invoke('stop_note', { note });
  } catch (error) {
    errorMessage.value = String(error);
  }
}

function handlePointerDown(note: number, event: PointerEvent) {
  if (event.button !== 0) return;

  pointerDown.value = true;
  startNote(note);
}

function handlePointerEnter(note: number, event: PointerEvent) {
  if (pointerDown.value && (event.buttons & 1) === 1) startNote(note);
}

async function releasePointer() {
  if (!pointerDown.value) return;

  pointerDown.value = false;
  await Promise.all([...activeNotes.value].map((note) => stopNote(note)));
}

async function changeOctave(direction: -1 | 1) {
  const nextOffset = octaveOffset.value + direction;
  if (nextOffset < -2 || nextOffset > 3) return;

  await Promise.all([...activeNotes.value].map((note) => stopNote(note)));
  octaveOffset.value = nextOffset;
}

async function selectWaveform() {
  errorMessage.value = '';
  try {
    await invoke('select_waveform', { waveform: selectedWaveform.value });
    await loadWaveform();
  } catch (error) {
    errorMessage.value = String(error);
  }
}

async function loadWaveform() {
  try {
    waveformSamples.value = await invoke<number[]>('get_waveform', {
      waveform: selectedWaveform.value,
    });
  } catch (error) {
    errorMessage.value = String(error);
  }
}

const waveformPoints = computed(() =>
  waveformSamples.value
    .map((sample, index, samples) => {
      const x = (index / (samples.length - 1)) * 600;
      const y = 120 - (sample / 32768) * 100;
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(' '),
);

const audioOutputPoints = computed(() =>
  audioOutputSamples.value
    .map((sample, index, samples) => {
      const x = (index / (samples.length - 1)) * 600;
      const y = Math.max(8, Math.min(232, 120 - (sample / 32768) * 170));
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(' '),
);

async function loadAudioOutput() {
  try {
    audioOutputSamples.value = await invoke<number[]>('get_audio_output');
  } catch (error) {
    errorMessage.value = String(error);
  }
}

onMounted(() => {
  loadWaveform();
  loadAudioOutput();
  outputTimer = window.setInterval(loadAudioOutput, 50);
  window.addEventListener('pointerup', releasePointer);
  window.addEventListener('pointercancel', releasePointer);
});

onUnmounted(() => {
  if (outputTimer !== undefined) window.clearInterval(outputTimer);
  window.removeEventListener('pointerup', releasePointer);
  window.removeEventListener('pointercancel', releasePointer);
});
</script>

<template>
  <div id="midi" class="hst-app-background max-h-screen min-h-screen overflow-y-auto px-4 py-4 sm:px-5 lg:px-6">
    <div class="mx-auto flex max-w-6xl flex-col gap-4">
      <header class="flex items-center justify-between rounded-2xl border border-[rgba(148,163,184,0.22)] bg-[rgba(15,23,42,0.8)] px-4 py-3 shadow-[0_0_0_1px_rgba(110,168,254,0.06),0_18px_40px_rgba(3,7,18,0.4)] backdrop-blur-sm">
        <div class="flex items-center gap-4">
          <div class="flex h-10 w-10 items-center justify-center rounded-xl border border-[rgba(110,168,254,0.32)] bg-[rgba(110,168,254,0.12)] p-1.5">
            <img :src="logo" alt="Hi Squeaky Things logo" class="h-full w-full object-contain" />
          </div>
          <div>
            <h1 class="mt-1 text-xl font-semibold tracking-[0.12em] text-white sm:text-2xl">Squeaky Machine</h1>
          </div>
        </div>

    
      </header>

 

      <section class="grid gap-4 md:grid-cols-[1.6fr_0.9fr]">
        <div class="rounded-2xl border border-[rgba(148,163,184,0.22)] bg-[rgba(15,23,42,0.82)] p-4 shadow-[0_18px_40px_rgba(3,7,18,0.3)]">
          <div class="mb-2 flex items-center justify-between gap-3">
            <div>
              <h2 class="mt-2 text-lg font-medium text-white">Waveform</h2>
            </div>
            <div class="rounded-full border border-[rgba(110,168,254,0.22)] bg-[rgba(110,168,254,0.1)] px-3 py-1 text-[10px] uppercase tracking-[0.2em] text-[#cfe1ff]">
              {{ selectedWaveform }} / {{ waveformNames.length - 1 }}
            </div>
          </div>

          <label class="mb-2 block" for="waveform">
            <span class="mb-2 block text-[10px] uppercase tracking-[0.24em] text-slate-400">Choose your waveform</span>
            <select
              id="waveform"
              v-model="selectedWaveform"
              class="w-full rounded-xl border border-[rgba(148,163,184,0.22)] bg-[#0b1324] px-3 py-2.5 text-sm text-slate-100 outline-none transition focus:border-[rgba(110,168,254,0.9)] focus:ring-2 focus:ring-[rgba(110,168,254,0.2)]"
              @change="selectWaveform"
            >
              <option v-for="waveform in waveforms" :key="waveform.value" :value="waveform.value">
                {{ waveform.label }}
              </option>
            </select>
          </label>

          <div class="mb-5 rounded-xl border border-[rgba(148,163,184,0.18)] bg-[rgba(11,19,36,0.9)] p-3">
            <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.24em] text-slate-400">
              <span>Signal</span>
              <span>{{ selectedWaveformLabel }}</span>
            </div>
            <svg
              class="h-20 w-full overflow-visible"
              viewBox="0 0 600 240"
              preserveAspectRatio="none"
              role="img"
              :aria-label="`Waveform ${selectedWaveform} graph`"
            >
              <line x1="0" y1="120" x2="600" y2="120" stroke="rgba(148,163,184,0.25)" stroke-width="1" />
              <polyline
                v-if="waveformPoints"
                :points="waveformPoints"
                fill="none"
                stroke="#6ea8fe"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
              />
            </svg>
          </div>
        </div>

        <aside class="rounded-2xl border border-[rgba(148,163,184,0.22)] bg-[rgba(15,23,42,0.82)] p-4 shadow-[0_18px_40px_rgba(3,7,18,0.3)]">
    
      <h2 class="mt-2 text-lg font-medium text-white">Output</h2>

          <div class="mt-4 grid grid-cols-3 gap-3">
            <div class="rounded-xl border border-[rgba(148,163,184,0.18)] bg-[rgba(11,19,36,0.9)] p-3">
              <div class="text-[9px] uppercase tracking-[0.2em] text-slate-400">Voices</div>
              <div class="mt-2 text-2xl font-semibold text-white">{{ activeVoiceCount }}</div>
            </div>
            <div class="rounded-xl border border-[rgba(148,163,184,0.18)] bg-[rgba(11,19,36,0.9)] p-3">
              <div class="text-[9px] uppercase tracking-[0.2em] text-slate-400">Mode</div>
              <div class="mt-2 text-sm font-semibold text-[#cfe1ff]">LIVE</div>
            </div>
            <div class="rounded-xl border border-[rgba(148,163,184,0.18)] bg-[rgba(11,19,36,0.9)] p-3">
              <div class="text-[9px] uppercase tracking-[0.2em] text-slate-400">Gain</div>
              <div class="mt-2 text-sm font-semibold text-[#cfe1ff]">100%</div>
            </div>
          </div>

          <div class="mt-3 rounded-xl border border-[rgba(148,163,184,0.18)] bg-[rgba(11,19,36,0.9)] p-3">
            <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.24em] text-slate-400">
              <span>Live</span>
              <span>Realtime</span>
            </div>
            <svg
              class="h-20 w-full overflow-visible"
              viewBox="0 0 600 240"
              preserveAspectRatio="none"
              role="img"
              aria-label="Live synthesizer output graph"
            >
              <line x1="0" y1="120" x2="600" y2="120" stroke="rgba(148,163,184,0.25)" stroke-width="1" />
              <polyline
                v-if="audioOutputPoints"
                :points="audioOutputPoints"
                fill="none"
                stroke="#86efac"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
              />
            </svg>
          </div>
        </aside>
      </section>

           <section class="rounded-2xl border border-[rgba(148,163,184,0.22)] bg-[rgba(15,23,42,0.82)] p-4 shadow-[0_18px_40px_rgba(3,7,18,0.3)]">

        <div class="relative flex h-36 min-w-0 overflow-hidden rounded-xl border border-[rgba(148,163,184,0.28)] bg-[#070c16] p-1 sm:h-40">
          <button
            v-for="key in whiteKeys"
            :key="key.note"
            type="button"
            class="relative z-0 h-full min-w-0 flex-1 rounded-b-md border border-slate-300 bg-slate-100 pb-3 text-left align-bottom text-[10px] font-semibold text-slate-500 shadow-[inset_0_-8px_0_rgba(15,23,42,0.08),0_3px_0_rgba(15,23,42,0.55)] transition active:translate-y-0.5 active:shadow-[inset_0_-3px_0_rgba(15,23,42,0.12)]"
            :class="activeNotes.has(key.note) ? 'bg-[#9ec2ff] text-slate-900' : 'hover:bg-white'"
            :aria-label="`Play ${key.label}`"
            @pointerdown.prevent="handlePointerDown(key.note, $event)"
            @pointerenter="handlePointerEnter(key.note, $event)"
            @pointerup="stopNote(key.note)"
            @pointercancel="stopNote(key.note)"
            @pointerleave="stopNote(key.note)"
            @contextmenu.prevent
          >
            <span class="absolute bottom-2 left-2">{{ key.label }}</span>
          </button>

          <button
            v-for="key in blackKeys"
            :key="key.note"
            type="button"
            class="absolute top-1 z-10 h-[62%] w-10 -translate-x-1/2 rounded-b-md border border-slate-950 bg-slate-900 text-left text-[9px] font-semibold text-slate-400 shadow-[0_5px_0_rgba(2,6,23,0.8),inset_0_-5px_0_rgba(148,163,184,0.12)] transition active:translate-y-0.5"
            :class="activeNotes.has(key.note) ? 'bg-[#315b91] text-white' : 'hover:bg-slate-800'"
            :style="{ left: key.position }"
            :aria-label="`Play ${key.label}`"
            @pointerdown.prevent="handlePointerDown(key.note, $event)"
            @pointerenter="handlePointerEnter(key.note, $event)"
            @pointerup="stopNote(key.note)"
            @pointercancel="stopNote(key.note)"
            @pointerleave="stopNote(key.note)"
            @contextmenu.prevent
          >
            <span class="absolute bottom-2 left-1/2 -translate-x-1/2 whitespace-nowrap">{{ key.label }}</span>
          </button>
        </div>

        <div class="mt-3 flex items-center justify-center gap-3">
          <button
            type="button"
            class="rounded-lg border border-[rgba(148,163,184,0.28)] bg-[rgba(11,19,36,0.8)] px-3 py-1.5 text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-300 transition hover:border-[rgba(110,168,254,0.7)] hover:text-white disabled:cursor-not-allowed disabled:opacity-35"
            :disabled="octaveOffset <= -2"
            @click="changeOctave(-1)"
          >
            Octave down
          </button>
          <span class="min-w-20 text-center text-[10px] uppercase tracking-[0.18em] text-slate-400">
            {{ octaveOffset === 0 ? 'Default' : `${octaveOffset > 0 ? '+' : ''}${octaveOffset}` }}
          </span>
          <button
            type="button"
            class="rounded-lg border border-[rgba(148,163,184,0.28)] bg-[rgba(11,19,36,0.8)] px-3 py-1.5 text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-300 transition hover:border-[rgba(110,168,254,0.7)] hover:text-white disabled:cursor-not-allowed disabled:opacity-35"
            :disabled="octaveOffset >= 3"
            @click="changeOctave(1)"
          >
            Octave up
          </button>
        </div>
      </section>
      <div v-if="errorMessage" class="rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-200">
        {{ errorMessage }}
      </div>
    </div>
  </div>
</template>
