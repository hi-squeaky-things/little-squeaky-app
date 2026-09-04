<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import logo from './assets/logo/logo.png';

type PatchInfo = {
  name: string;
  mode: 'Mono' | 'BiPoly' | 'QuadPoly' | 'OctoPoly';
};

const activeNotes = ref(new Set<number>());
const heldNoteGroups = ref(new Map<number, number[]>());
const errorMessage = ref('');
const patches = ref<PatchInfo[]>([]);
const selectedPatch = ref(0);
const chordMode = ref(false);
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
  'sin',
  'saw',
  'squ',
  'tri',
  'ebass',
  'cello',
  'violin',
  'eorgan',
  'epiano',
  'overtone',
  'granular_0001',
  'granular_0002',
  'granular_0003',
  'granular_0004',
  'granular_0005',
  'granular_0006',
  'granular_0007',
  'granular_0008',
  'granular_0009',
  'granular_0010',
  'granular_0011',
  'granular_0012',
  'granular_0013',
  'granular_0014',
  'granular_0015',
  'granular_0016',
  'granular_0017',
  'granular_0018',
  'granular_0019',
  'granular_0020',
  'granular_0021',
  'granular_0022',
  'granular_0023',
  'granular_0024',
  'granular_0025',
  'granular_0026',
  'granular_0027',
  'granular_0028',
  'granular_0029',
  'granular_0030',
  'granular_0031',
  'granular_0032',
  'granular_0033',
  'granular_0034',
  'granular_0035',
  'granular_0036',
  'granular_0037',
  'granular_0038',
  'granular_0039',
  'granular_0040',
  'granular_0041',
  'granular_0042',
  'granular_0043',
  'granular_0044',
  'contra_bass',
  'slap_bass',
];

const waveforms = waveformNames.map((label, value) => ({ label, value }));

const activeVoiceCount = computed(() => activeNotes.value.size);
const selectedPatchInfo = computed(() => patches.value[selectedPatch.value]);
const chordAvailable = computed(
  () => selectedPatchInfo.value !== undefined && selectedPatchInfo.value.mode !== 'Mono',
);
const selectedWaveformLabel = computed(
  () => waveformNames[selectedWaveform.value] ?? waveformNames[0],
);
const selectedPatchLabel = computed(
  () => selectedPatchInfo.value?.name ?? 'Piano',
);

function refreshActiveNotes() {
  activeNotes.value = new Set([...heldNoteGroups.value.values()].flat());
}

async function startNote(note: number) {
  if (heldNoteGroups.value.has(note)) return;

  const notes = chordMode.value && chordAvailable.value ? [note, note + 4, note + 7] : [note];
  if (notes.some((chordNote) => chordNote < 24 || chordNote > 108)) {
    errorMessage.value = 'Chord is outside the MIDI range';
    return;
  }

  errorMessage.value = '';
  heldNoteGroups.value = new Map(heldNoteGroups.value).set(note, notes);
  refreshActiveNotes();
  try {
    await Promise.all(notes.map((chordNote) => invoke('play_note', { note: chordNote, velocity: 100 })));
  } catch (error) {
    const groups = new Map(heldNoteGroups.value);
    groups.delete(note);
    heldNoteGroups.value = groups;
    refreshActiveNotes();
    await Promise.all(notes.map((chordNote) => invoke('stop_note', { note: chordNote }).catch(() => undefined)));
    errorMessage.value = String(error);
  }
}

async function stopNote(note: number) {
  const notes = heldNoteGroups.value.get(note);
  if (!notes) return;

  const groups = new Map(heldNoteGroups.value);
  groups.delete(note);
  heldNoteGroups.value = groups;
  const remainingNotes = new Set([...groups.values()].flat());
  const notesToStop = notes.filter((chordNote) => !remainingNotes.has(chordNote));
  refreshActiveNotes();
  await Promise.all(notesToStop.map((chordNote) => invoke('stop_note', { note: chordNote }).catch((error) => {
    errorMessage.value = String(error);
  })));
}

async function stopAllNotes() {
  const notes = [...new Set([...heldNoteGroups.value.values()].flat())];
  heldNoteGroups.value = new Map();
  activeNotes.value = new Set();
  await Promise.all(notes.map((note) => invoke('stop_note', { note }).catch((error) => {
    errorMessage.value = String(error);
  })));
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
  await stopAllNotes();
}

async function changeOctave(direction: -1 | 1) {
  const nextOffset = octaveOffset.value + direction;
  if (nextOffset < -2 || nextOffset > 3) return;

  await stopAllNotes();
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

async function loadPatches() {
  try {
    patches.value = await invoke<PatchInfo[]>('get_patches');
  } catch (error) {
    errorMessage.value = String(error);
  }
}

async function selectPatch() {
  errorMessage.value = '';
  await stopAllNotes();
  if (!chordAvailable.value) chordMode.value = false;
  try {
    await invoke('select_patch', { patch: selectedPatch.value });
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
  loadPatches();
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
  stopAllNotes();
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
            <h1 class="mt-1 text-xl font-semibold tracking-[0.12em] text-white sm:text-2xl">Little Squeaky</h1>
          </div>
        </div>

    
      </header>

 

      <section class="grid gap-4 md:grid-cols-[1.6fr_0.9fr]">
        <div class="rounded-2xl border border-[rgba(148,163,184,0.22)] bg-[rgba(15,23,42,0.82)] p-4 shadow-[0_18px_40px_rgba(3,7,18,0.3)]">
          <div class="mb-2 flex items-center justify-between gap-3">
            <div>
              <h2 class="mt-2 text-lg font-medium text-white">Patch & Waveform</h2>
            </div>
            <div class="rounded-full border border-[rgba(110,168,254,0.22)] bg-[rgba(110,168,254,0.1)] px-3 py-1 text-[10px] uppercase tracking-[0.2em] text-[#cfe1ff]">
              {{ selectedPatchLabel }}
            </div>
          </div>

          <label class="mb-2 block" for="patch">
            <span class="mb-2 block text-[10px] uppercase tracking-[0.24em] text-slate-400">Choose your patch</span>
            <select
              id="patch"
              v-model="selectedPatch"
              class="w-full rounded-xl border border-[rgba(148,163,184,0.22)] bg-[#0b1324] px-3 py-2.5 text-sm text-slate-100 outline-none transition focus:border-[rgba(110,168,254,0.9)] focus:ring-2 focus:ring-[rgba(110,168,254,0.2)]"
              @change="selectPatch"
            >
              <option v-for="(patch, index) in patches" :key="patch.name" :value="index">
                {{ patch.name }}
              </option>
            </select>
          </label>

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
            :class="chordMode ? 'border-[#6ea8fe] bg-[rgba(110,168,254,0.18)] text-white' : ''"
            :disabled="!chordAvailable"
            :aria-pressed="chordMode"
            :title="chordAvailable ? 'Play major chords' : 'Chord mode is unavailable for Mono patches'"
            @click="chordMode = !chordMode"
          >
            Chord {{ chordMode ? 'on' : 'off' }}
          </button>
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
