// A Hex user does the meeting work by hand, then Hex Meetings does it.
// One timeline: every frame is a pure function of the time `t`, so the show can be played, replayed
// or opened at any moment (?t=12). The sound is synthesized live.

type Marks = Record<"icon" | "slack" | "notion" | "github" | "linear" | "opencode" | "claude" | "codex" | "cursor" | "pi", string>;

/** The moment Hex Meetings lands and the mess shatters. */
const R = 19.6;
const END = R + 11;

const clamp = (x: number, a = 0, b = 1) => Math.min(b, Math.max(a, x));
const inOut = (x: number) => (x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2);
const out = (x: number) => 1 - (1 - x) ** 3;
const back = (x: number) => 1 + 2.2 * (x - 1) ** 3 + 1.2 * (x - 1) ** 2;
const seg = (t: number, a: number, b: number, ease = inOut) => ease(clamp((t - a) / (b - a)));
const mix = (a: number, b: number, p: number) => a + (b - a) * p;

function random(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let x = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    x = (x + Math.imul(x ^ (x >>> 7), 61 | x)) ^ x;
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296;
  };
}

const chevron = `<svg class="chev" viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M3 4.5 6 7.5 9 4.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

const glyphs = {
  // A plain hexagon for Hex's own window; not Hex's logo.
  hex: `<svg viewBox="0 0 20 20" fill="none" stroke="#cfcfd4" stroke-width="1.6"><path d="M10 2.2 16.8 6.1v7.8L10 17.8 3.2 13.9V6.1z"/></svg>`,
  mail: `<svg viewBox="0 0 20 20" fill="none" stroke="#8fb0ff" stroke-width="1.6"><rect x="2.5" y="4.5" width="15" height="11" rx="2"/><path d="m3 5.5 7 5.5 7-5.5"/></svg>`,
  cal: `<svg viewBox="0 0 20 20" fill="none" stroke="#ff8589" stroke-width="1.6"><rect x="3" y="4" width="14" height="13" rx="2"/><path d="M3 8h14M7 2.5v3M13 2.5v3"/></svg>`,
  chat: `<svg viewBox="0 0 20 20" fill="none" stroke="#cfcfd4" stroke-width="1.6"><path d="M4 4.5h12v8H9l-4 3v-3H4z"/></svg>`,
  copy: `<svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="7" y="7" width="10" height="10" rx="2"/><path d="M13 4H5a1 1 0 0 0-1 1v8"/></svg>`,
  soundOn: `<svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 8v4h3l4 3V5L6 8z" fill="currentColor"/><path d="M13 7.5a3.5 3.5 0 0 1 0 5M15.2 5.3a6.6 6.6 0 0 1 0 9.4"/></svg>`,
  soundOff: `<svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 8v4h3l4 3V5L6 8z" fill="currentColor"/><path d="m13.5 8 4 4m0-4-4 4"/></svg>`,
  check: `<svg viewBox="0 0 20 20" width="18" height="18" style="position:absolute;inset:-1.5px"><path d="M5.5 10.5 8.5 13.5 14.5 7" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" pathLength="1"/></svg>`,
};

// What Hex gives you: the call, its transcript (You and Them) and a short summary.
const LINES: [string, string][] = [
  ["Them", "We'd want 20 seats this quarter."],
  ["Them", "Exports to CSV fail on big accounts."],
  ["You", "I'll send a pilot plan by Friday."],
  ["Them", "Can you share notes with the team?"],
];
const PROMPT = "Summarize this call with action items and owners, formatted for Slack.";

type App = "hex" | "mail" | "slack" | "cal" | "notion" | "github";
// While you copy and paste one call, the next ones arrive.
const NOTES: [number, App, string, string, number, number, number, number?][] = [
  [12.6, "hex", "New in Hex", "Fabrikam: export failing, 18 min", 200, 110, -3],
  [13.2, "cal", "Calendar", "Team standup in 5 min", 1080, 400, 2],
  [13.8, "slack", "#sales, Dax", "Notes from the Northwind call?", 450, 100, -2],
  [14.4, "hex", "New in Hex", "Team standup, 12 min", 200, 330, 2],
  [14.9, "mail", "Priya, Northwind", "Following up on next steps", 830, 110, 2],
  [15.4, "github", "fabrikam/exporter", "#212: can you add steps to reproduce?", 470, 420, -2, 2],
  [15.9, "hex", "New in Hex", "Board prep with Alex, 47 min", 1080, 160, -3],
  [16.3, "slack", "#sales, Kit", "Where's the Northwind transcript?", 760, 300, 3, 3],
  [16.7, "cal", "Calendar", "Design candidate in 10 min", 160, 520, -4],
  [17.0, "hex", "New in Hex", "Design candidate, round 2, 41 min", 640, 200, 2],
  [17.3, "notion", "Notion", "Northwind deal note: still empty", 960, 530, -2],
  [17.6, "mail", "Priya, Northwind", "Any update?", 360, 250, 4, 2],
  [17.9, "slack", "#sales, Dax", "@you?", 1100, 280, -2, 4],
];

const CAPTIONS: [number, number, string][] = [
  [0.4, 4.2, "Hex records and transcribes the call."],
  [4.6, 9.8, "Then it's copy, paste and prompt."],
  [10.2, 14.6, "Then again for Slack, Notion and GitHub."],
  [14.8, R - 0.2, "And again for every call after it."],
  [R + 1.0, R + 3.8, "Hex Meetings picks up every call Hex records."],
  [R + 4.0, R + 6.6, "Save a report once. Run it with your own agent."],
  [R + 6.8, END, "Read it, then send it to Slack, Notion, GitHub or Linear."],
];

const MEETINGS: [string, string][] = [
  ["Northwind intro call", "Google Meet, 34 min"],
  ["Fabrikam: export failing", "Zoom, 18 min"],
  ["Team standup", "Slack, 12 min"],
  ["Board prep with Alex", "47 min"],
  ["Design candidate, round 2", "Microsoft Teams, 41 min"],
];
const ITEMS = ["You: send the pilot plan by Friday", "You: look into the CSV export bug", "You: share notes in #sales"];
// The agents a report can run with, in the order the picker shows them here.
const AGENT_ORDER = ["claude", "codex", "cursor", "pi", "opencode"] as const;
const AGENT_NAMES = { claude: "Claude Code", codex: "Codex", cursor: "Cursor Agent", pi: "Pi", opencode: "OpenCode" };
const AGENT_AT = (i: number) => R + 2.6 + i * 0.4;

type Place = { o?: number; s?: number; r?: number };
function place(el: HTMLElement, x: number, y: number, { o = 1, s = 1, r = 0 }: Place = {}) {
  el.style.transform = `translate(${x}px, ${y}px) translate(-50%, -50%) rotate(${r}deg) scale(${s})`;
  el.style.opacity = String(o);
  el.style.visibility = o <= 0.001 ? "hidden" : "visible";
}

/** A short burst of decaying noise, used as the room the sounds play in. */
function room(c: BaseAudioContext, seconds: number, decay: number) {
  const len = Math.ceil(c.sampleRate * seconds);
  const buf = c.createBuffer(2, len, c.sampleRate);
  for (let ch = 0; ch < 2; ch++) {
    const d = buf.getChannelData(ch);
    for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / len) ** decay;
  }
  return buf;
}

/**
 * Live synthesized sound. Everything runs through a small room reverb and a limiter, so the impact
 * can be loud without clipping. `at` schedules sounds at a set time instead of now, for offline renders.
 */
export class Sound {
  ctx: BaseAudioContext | null = null;
  bus: GainNode | null = null;
  volume: GainNode | null = null;
  drone: { stop: (at: number) => void } | null = null;
  muted = false;
  at: number | null = null;

  private now() {
    return this.at ?? this.ctx!.currentTime;
  }
  /** Made on intent (hover or focus), so pressing play doesn't wait for it. */
  prepare() {
    if (this.ctx) return;
    try {
      this.build(new AudioContext({ latencyHint: "interactive" }));
    } catch {
      /* No sound on this browser; the show still plays. */
    }
  }
  start() {
    this.prepare();
    void (this.ctx as AudioContext | null)?.resume();
  }
  build(c: BaseAudioContext) {
    this.ctx = c;
    const limiter = c.createDynamicsCompressor();
    limiter.threshold.value = -8;
    limiter.knee.value = 6;
    limiter.ratio.value = 10;
    limiter.attack.value = 0.003;
    limiter.release.value = 0.25;
    this.volume = c.createGain();
    this.volume.gain.value = this.muted ? 0 : 0.9;
    this.volume.connect(limiter).connect(c.destination);
    this.bus = c.createGain();
    this.bus.connect(this.volume);
    const verb = c.createConvolver();
    verb.buffer = room(c, 1.8, 3);
    const wet = c.createGain();
    wet.gain.value = 0.22;
    this.bus.connect(verb).connect(wet).connect(this.volume);
  }
  setMuted(m: boolean) {
    this.muted = m;
    if (this.ctx && this.volume) this.volume.gain.setTargetAtTime(m ? 0 : 0.9, this.ctx.currentTime, 0.02);
  }
  private voice(vol: number, attack: number, dur: number, when: number, pan = 0) {
    const c = this.ctx!;
    const g = c.createGain();
    const t = this.now() + when;
    g.gain.setValueAtTime(0, t);
    g.gain.linearRampToValueAtTime(vol, t + attack);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    if (pan) {
      const p = c.createStereoPanner();
      p.pan.value = pan;
      g.connect(p).connect(this.bus!);
    } else g.connect(this.bus!);
    return { g, t };
  }
  tone(f: number, dur: number, type: OscillatorType, vol: number, when = 0, to?: number, attack = 0.004, pan = 0) {
    if (!this.ctx) return;
    const { g, t } = this.voice(vol, attack, dur, when, pan);
    const o = this.ctx.createOscillator();
    o.type = type;
    o.frequency.setValueAtTime(f, t);
    if (to) o.frequency.exponentialRampToValueAtTime(to, t + dur);
    o.connect(g);
    o.start(t);
    o.stop(t + dur + 0.05);
  }
  noise(dur: number, vol: number, type: BiquadFilterType, f: number, to?: number, q = 1, attack = 0.01, pan?: [number, number]) {
    if (!this.ctx) return;
    const c = this.ctx;
    const buf = c.createBuffer(1, Math.ceil(c.sampleRate * dur), c.sampleRate);
    const d = buf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    const src = c.createBufferSource();
    src.buffer = buf;
    const bq = c.createBiquadFilter();
    bq.type = type;
    bq.Q.value = q;
    const { g, t } = this.voice(vol, attack, dur, 0);
    bq.frequency.setValueAtTime(f, t);
    if (to) bq.frequency.exponentialRampToValueAtTime(to, t + dur);
    if (pan) {
      const p = c.createStereoPanner();
      p.pan.setValueAtTime(pan[0], t);
      p.pan.linearRampToValueAtTime(pan[1], t + dur);
      src.connect(bq).connect(p).connect(g);
    } else src.connect(bq).connect(g);
    src.start(t);
  }
  /** A soft bell: the note and a quiet octave above it. */
  bell(f: number, vol: number, dur: number, when = 0, pan = 0) {
    this.tone(f, dur, "sine", vol, when, undefined, 0.004, pan);
    this.tone(f * 2, dur * 0.5, "sine", vol * 0.25, when, undefined, 0.004, pan);
  }
  /** A notification: two bell notes a fifth apart. */
  ping(f: number, pan: number) {
    this.bell(f, 0.16, 0.45, 0, pan);
    this.bell(f * 1.5, 0.11, 0.55, 0.08, pan);
  }
  /** A card settling in. */
  appear(pan = 0) {
    this.tone(280, 0.14, "sine", 0.12, 0, 560, 0.004, pan);
  }
  key(vol = 0.1) {
    this.noise(0.025, vol, "bandpass", 2600 + Math.random() * 1400, undefined, 1.2, 0.002);
  }
  click() {
    this.noise(0.02, 0.09, "bandpass", 1800, undefined, 1, 0.001);
    this.tone(180, 0.06, "sine", 0.12);
  }
  /** A line flying across the stage, left to right. */
  whoosh() {
    this.noise(0.45, 0.24, "bandpass", 300, 3000, 0.8, 0.12, [-0.6, 0.6]);
  }
  error() {
    this.tone(440, 0.14, "triangle", 0.1);
    this.tone(311, 0.24, "triangle", 0.1, 0.13);
  }
  /** A low hum that rises a fifth and pulses faster as the questions pile up. */
  droneOn(seconds: number) {
    if (!this.ctx || this.drone) return;
    const c = this.ctx;
    const t = this.now();
    const g = c.createGain();
    g.gain.setValueAtTime(0.02, t);
    g.gain.linearRampToValueAtTime(0.3, t + seconds);
    const pulse = c.createGain();
    pulse.gain.value = 0.7;
    const lfo = c.createOscillator();
    lfo.frequency.setValueAtTime(2, t);
    lfo.frequency.exponentialRampToValueAtTime(9, t + seconds);
    const depth = c.createGain();
    depth.gain.value = 0.3;
    lfo.connect(depth).connect(pulse.gain);
    const lp = c.createBiquadFilter();
    lp.type = "lowpass";
    lp.Q.value = 2;
    lp.frequency.setValueAtTime(500, t);
    lp.frequency.exponentialRampToValueAtTime(2600, t + seconds);
    lp.connect(pulse).connect(g).connect(this.bus!);
    const oscs = [110, 111.3, 164.8].map((f) => {
      const o = c.createOscillator();
      o.type = "sawtooth";
      o.frequency.setValueAtTime(f, t);
      o.frequency.exponentialRampToValueAtTime(f * 1.5, t + seconds);
      o.connect(lp);
      o.start(t);
      return o;
    });
    lfo.start(t);
    this.drone = {
      stop: (at) => {
        g.gain.cancelScheduledValues(at);
        g.gain.setValueAtTime(0.3, at);
        g.gain.linearRampToValueAtTime(0, at + 0.04);
        [...oscs, lfo].forEach((o) => o.stop(at + 0.1));
      },
    };
  }
  stopDrone() {
    if (this.ctx && this.drone) this.drone.stop(this.now());
    this.drone = null;
  }
  /** Noise that rises into the impact. */
  riser(seconds: number) {
    if (!this.ctx) return;
    const c = this.ctx;
    const t = this.now();
    const buf = c.createBuffer(1, Math.ceil(c.sampleRate * seconds), c.sampleRate);
    const d = buf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    const src = c.createBufferSource();
    src.buffer = buf;
    const bq = c.createBiquadFilter();
    bq.type = "bandpass";
    bq.Q.value = 1.5;
    bq.frequency.setValueAtTime(400, t);
    bq.frequency.exponentialRampToValueAtTime(7000, t + seconds);
    const g = c.createGain();
    g.gain.setValueAtTime(0.01, t);
    g.gain.linearRampToValueAtTime(0.24, t + seconds - 0.02);
    g.gain.linearRampToValueAtTime(0, t + seconds);
    src.connect(bq).connect(g).connect(this.bus!);
    src.start(t);
  }
  impact() {
    this.stopDrone();
    this.tone(110, 1.1, "sine", 0.85, 0, 34, 0.003);
    this.noise(0.8, 0.35, "lowpass", 3000, 180, 0.7, 0.003);
    for (let i = 0; i < 10; i++) {
      this.tone(2500 + Math.random() * 3500, 0.12, "sine", 0.05, 0.03 + i * 0.045, undefined, 0.002, Math.random() * 1.6 - 0.8);
    }
  }
  /** The resolution: a major seventh, rolled upward, and a high sparkle. */
  chime(vol = 0.08) {
    [523.25, 659.25, 783.99, 987.77, 1046.5].forEach((f, i) => this.tone(f, 2.2, "sine", vol, i * 0.07, undefined, 0.02));
  }
  /** A message on its way. */
  send() {
    this.noise(0.3, 0.1, "bandpass", 800, 4000, 0.9, 0.05, [0, 0.5]);
  }
}

type Cue = [number, () => void];
const pan = (x: number) => ((x - 640) / 640) * 0.7;

// Where the copied text goes by hand, and when.
const HAND = { slack: { x: 250, y: 500, at: 10.2 }, notion: { x: 640, y: 520, at: 10.9 }, github: { x: 1030, y: 500, at: 11.6 } };

/** Every sound in the show, at the moment it plays. */
function cues(sound: Sound): Cue[] {
  return [
    [0.35, () => sound.appear()],
    ...LINES.map((_, i): Cue => [1.2 + i * 0.6, () => sound.key()]),
    [4.6, () => sound.appear(0.5)],
    ...Array.from({ length: 30 }, (_, i): Cue => [5.0 + i * 0.06, () => sound.key(0.08)]),
    ...LINES.map((_, i): Cue => [6.9 + i * 0.3, () => sound.whoosh()]),
    ...Array.from({ length: 10 }, (_, i): Cue => [8.3 + i * 0.1, () => sound.key(0.04)]),
    [9.6, () => sound.click()],
    [9.65, () => sound.bell(1318.5, 0.06, 0.25, 0, 0.5)],
    ...Object.values(HAND).flatMap(({ x, at }): Cue[] => [
      [at, () => sound.appear(pan(x))],
      [at + 0.1, () => sound.whoosh()],
      ...Array.from({ length: 8 }, (_, i): Cue => [at + 0.45 + i * 0.05, () => sound.key(0.06)]),
    ]),
    [12.5, () => sound.droneOn(R - 12.5)],
    ...NOTES.map(([at, , , , x], i): Cue => [at, () => sound.ping(660 * 2 ** (i / 12), pan(x))]),
    [R - 1.4, () => sound.riser(1.4)],
    [R, () => sound.impact()],
    [R + 0.12, () => sound.chime()],
    [R + 1.4, () => sound.appear(-0.5)],
    [R + 2.0, () => sound.appear()],
    ...AGENT_ORDER.map((_, i): Cue => [AGENT_AT(i), () => sound.bell(880 * 2 ** (i / 6), 0.05, 0.2)]),
    [R + 4.8, () => sound.click()],
    ...Array.from({ length: 14 }, (_, i): Cue => [R + 5.2 + i * 0.05, () => sound.key(0.05)]),
    ...ITEMS.map((_, i): Cue => [R + 6.05 + i * 0.35, () => sound.bell([1046.5, 1318.5, 1568][i]!, 0.1, 0.4)]),
    [R + 6.9, () => sound.appear(0.5)],
    [R + 8.3, () => sound.click()],
    [R + 8.32, () => sound.send()],
    [R + 8.45, () => sound.chime(0.06)],
    ...[0, 1, 2].map((i): Cue => [R + 9.0 + i * 0.18, () => sound.appear(0.6)]),
  ];
}

/** The whole soundtrack as a WAV file (base64), rendered offline, for checking the mix. */
export async function renderSoundtrack() {
  const rate = 44100;
  const c = new OfflineAudioContext(2, Math.ceil(rate * (END + 2)), rate);
  const sound = new Sound();
  sound.build(c);
  for (const [at, cue] of cues(sound).sort((a, b) => a[0] - b[0])) {
    sound.at = at;
    cue();
  }
  const buf = await c.startRendering();
  const n = buf.length;
  const view = new DataView(new ArrayBuffer(44 + n * 4));
  const str = (o: number, v: string) => [...v].forEach((ch, i) => view.setUint8(o + i, ch.charCodeAt(0)));
  str(0, "RIFF");
  view.setUint32(4, 36 + n * 4, true);
  str(8, "WAVEfmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 2, true);
  view.setUint32(24, rate, true);
  view.setUint32(28, rate * 4, true);
  view.setUint16(32, 4, true);
  view.setUint16(34, 16, true);
  str(36, "data");
  view.setUint32(40, n * 4, true);
  const l = buf.getChannelData(0);
  const rr = buf.getChannelData(1);
  for (let i = 0; i < n; i++) {
    view.setInt16(44 + i * 4, clamp(l[i]!, -1, 1) * 32767, true);
    view.setInt16(46 + i * 4, clamp(rr[i]!, -1, 1) * 32767, true);
  }
  let bin = "";
  const bytes = new Uint8Array(view.buffer);
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(bin);
}

export async function mountShow() {
  const frame = document.querySelector<HTMLElement>("[data-frame]")!;
  const show = document.querySelector<HTMLElement>("[data-show]")!;
  const playKey = document.querySelector<HTMLButtonElement>("[data-play]")!;
  const intro = document.querySelector<HTMLElement>("[data-intro]")!;
  const hero = frame.closest<HTMLElement>(".hero");
  const controls = document.querySelector<HTMLElement>("[data-controls]")!;
  const muteButton = document.querySelector<HTMLButtonElement>("[data-mute]")!;
  const replay = document.querySelector<HTMLButtonElement>("[data-replay]")!;
  const marks: Marks = JSON.parse(show.dataset.marks!);

  new ResizeObserver(() => {
    show.style.setProperty("--s", String(frame.clientWidth / 1280));
    frame.style.setProperty("--h", `${frame.clientWidth / 2}px`);
  }).observe(frame);
  /** Opens the stage; the story starts once it has room. Returns the wait in seconds. */
  const open = (animate: boolean) => {
    const wait = frame.hasAttribute("data-open") || !animate ? 0 : 0.6;
    if (!animate) frame.style.transition = "none";
    frame.dataset.open = "";
    intro.dataset.gone = "";
    hero?.setAttribute("data-playing", "");
    return wait;
  };

  const add = (html: string) => {
    const t = document.createElement("template");
    t.innerHTML = html.trim();
    const el = t.content.firstElementChild as HTMLElement;
    show.append(el);
    return el;
  };
  const img = (src: string, size = 18) => `<img src="${src}" width="${size}" height="${size}" alt="">`;
  const typeIn = (el: HTMLElement, text: string, from: number, cps: number, t: number) => {
    el.textContent = text.slice(0, Math.max(0, Math.floor((t - from) * cps)));
  };

  // Built back to front.
  const vignette = add(`<div class="vignette"></div>`);
  // Hex: the call, its summary and transcript.
  const call = add(`<div class="card call"><div class="card__head">${glyphs.hex}<b>Hex</b><span>Meetings</span></div>
    <h3>Northwind intro call</h3><div class="meta">Google Meet, 34 min</div>
    <div class="hexsum"><span>Hex's summary</span>Northwind wants a pilot before Q1.</div>
    ${LINES.map(([who, said]) => `<div class="line line--${who.toLowerCase()}"><span class="who"><i></i>${who}</span><span>${said}</span></div>`).join("")}</div>`);
  const lines = [...call.querySelectorAll<HTMLElement>(".line")];

  // By hand: a chat for the summary, then each place it has to go.
  const chat = add(`<div class="card chat"><div class="card__head">${glyphs.chat}<b>AI chat</b><span>new chat</span></div>
    <div class="input"><span data-typed></span><br><span class="file" data-file>${glyphs.copy} Pasted transcript, 34 min</span></div>
    <div class="skel" data-skel><i style="width:92%"></i><i style="width:78%"></i><i style="width:64%"></i></div>
    <div class="panel__row"><span class="btn btn--quiet" data-copy style="margin-left:auto">${glyphs.copy} Copy</span></div></div>`);
  const prompt = chat.querySelector<HTMLElement>("[data-typed]")!;
  const pasted = chat.querySelector<HTMLElement>("[data-file]")!;
  const skel = [...chat.querySelectorAll<HTMLElement>("[data-skel] i")];
  const copy = chat.querySelector<HTMLElement>("[data-copy]")!;
  const fliers = LINES.map(([who, said]) => add(`<div class="flier"><b>${who}:</b> ${said.slice(0, 22)}…</div>`));

  const slackHand = add(`<div class="card"><div class="card__head">${img(marks.slack)}<b>#sales</b><span>Slack</span></div>
    <div class="input composer" data-text></div></div>`);
  const notionHand = add(`<div class="card"><div class="card__head">${img(marks.notion)}<b>Northwind deal note</b><span>Notion</span></div>
    <div class="composer" data-text></div></div>`);
  const githubHand = add(`<div class="card"><div class="card__head">${img(marks.github)}<b>New issue</b><span>fabrikam/exporter</span></div>
    <div class="card__title" data-title></div><div class="composer muted" data-text></div></div>`);
  const handText = {
    slack: [slackHand.querySelector<HTMLElement>("[data-text]")!, "Northwind call: 20 seats this quarter. Action items: pilot plan by Friday, CSV bug…"],
    notion: [notionHand.querySelector<HTMLElement>("[data-text]")!, "**Summary** Northwind wants 20 seats… **Action items** - pilot plan by Fri…"],
    github: [githubHand.querySelector<HTMLElement>("[data-text]")!, "Steps to reproduce: ?"],
  } as const;
  const githubTitle = githubHand.querySelector<HTMLElement>("[data-title]")!;
  const hands = { slack: slackHand, notion: notionHand, github: githubHand };
  const copied = Object.keys(HAND).map(() => add(`<div class="flier">${glyphs.copy} Copied</div>`));

  const icons: Record<App, string> = {
    hex: glyphs.hex,
    mail: glyphs.mail,
    cal: glyphs.cal,
    slack: img(marks.slack),
    notion: img(marks.notion),
    github: img(marks.github),
  };
  const notes = NOTES.map(([, app, who, text, , , , badge]) =>
    add(`<div class="card">${badge ? `<span class="badge">${badge}</span>` : ""}<div class="card__head">${icons[app]}<b>${who}</b><span>now</span></div><div class="card__body">${text}</div></div>`),
  );

  // Hex Meetings: every call, saved reports, your agent, and the places they go.
  const list = add(`<div class="card list"><div class="card__head">${img(marks.icon)}<b>Hex Meetings</b></div>
    ${MEETINGS.map(([title, meta], i) => `<div class="row${i === 0 ? " row--on" : ""}"><b>${title}</b><span>${meta}</span></div>`).join("")}</div>`);
  const panel = add(`<div class="card panel"><h3>Northwind intro call</h3><div class="meta">Google Meet, 34 min</div>
    <div class="label">Reports</div>
    <div class="panel__row wrap"><span class="chip" data-run>${img(marks.slack, 14)} Summary to Slack</span><span class="chip">${img(marks.notion, 14)} Sales call to Notion</span><span class="chip">${img(marks.github, 14)} Support call to issues</span></div>
    <div class="label">Agent</div>
    <div class="panel__row"><span class="chip agent"><span class="agent__face" data-face>${img(marks.claude, 14)} <span>Claude Code</span></span>${chevron}</span></div>
    <div class="result"><p class="wipe">Northwind wants 20 seats this quarter and a pilot plan by Friday.</p>
    <h4 class="wipe">Action items</h4>
    ${ITEMS.map((i) => `<div class="item"><span class="tick">${glyphs.check}</span><span class="wipe">${i}</span></div>`).join("")}</div></div>`);
  const run = panel.querySelector<HTMLElement>("[data-run]")!;
  const face = panel.querySelector<HTMLElement>("[data-face]")!;
  const wipes = [...panel.querySelectorAll<HTMLElement>(".wipe")];
  const items = [...panel.querySelectorAll<HTMLElement>(".item")];
  const preview = add(`<div class="card panel" style="width:380px"><div class="card__head" style="font-size:15px"><b>Summary to Slack</b><span class="ready" data-state>Ready to post</span></div>
    <div style="margin-top:10px"><b>Northwind intro call</b> (34 min)</div>
    <div class="card__body">Northwind wants 20 seats this quarter.</div>
    <ul class="bullets">${ITEMS.map((i) => `<li>${i}</li>`).join("")}</ul>
    <div class="panel__row">${img(marks.slack, 16)}<span class="meta" data-goes>Goes to Slack (#sales)</span>
    <span class="btn btn--quiet" data-skip style="margin-left:auto">Don't post</span><span class="btn" data-post style="margin-left:0">Post</span></div></div>`);
  const state = preview.querySelector<HTMLElement>("[data-state]")!;
  const goes = preview.querySelector<HTMLElement>("[data-goes]")!;
  const postButtons = [...preview.querySelectorAll<HTMLElement>("[data-skip], [data-post]")];
  const postButton = preview.querySelector<HTMLElement>("[data-post]")!;
  const tiles = (
    [
      [marks.notion, "Notion"],
      [marks.github, "GitHub"],
      [marks.linear, "Linear"],
    ] as const
  ).map(([src, name]) => add(`<div class="card tile">${img(src, 26)}<span>${name}</span></div>`));

  const flash = add(`<div class="flash"></div>`);
  const rings = add(`<svg class="rings" viewBox="0 0 1280 640">${[0, 1, 2].map(() => `<path/>`).join("")}</svg>`);
  const ringPaths = [...rings.querySelectorAll("path")];
  const mark = add(`<img class="mark" src="${marks.icon}" alt="">`);
  const captions = CAPTIONS.map(([, , text]) => add(`<div class="caption">${text}</div>`));

  // All the hand work shatters when Hex Meetings lands; the call itself carries on.
  const breakable: { el: HTMLElement; x: number; y: number; r: number }[] = [
    { el: chat, x: 1030, y: 230, r: 0 },
    ...(Object.keys(HAND) as (keyof typeof HAND)[]).map((k) => ({ el: hands[k], x: HAND[k].x, y: HAND[k].y, r: 0 })),
    ...notes.map((el, i) => ({ el, x: NOTES[i]![4], y: NOTES[i]![5], r: NOTES[i]![6] })),
  ];
  let shards: { el: HTMLElement; x: number; y: number; r: number; vx: number; vy: number; vr: number }[] | null = null;
  const shatter = () => {
    const rand = random(7);
    const made: NonNullable<typeof shards> = [];
    shards = made;
    for (const { el: card, x, y, r } of breakable) {
      const w = card.offsetWidth;
      const h = card.offsetHeight;
      const n = 3;
      const pt: [number, number][][] = [];
      for (let a = 0; a <= n; a++) {
        pt.push([]);
        for (let b = 0; b <= n; b++) {
          const edge = a === 0 || a === n || b === 0 || b === n;
          pt[a]!.push([(b / n) * 100 + (edge ? 0 : (rand() - 0.5) * 18), (a / n) * 100 + (edge ? 0 : (rand() - 0.5) * 18)]);
        }
      }
      for (let a = 0; a < n; a++) {
        for (let b = 0; b < n; b++) {
          const poly = [pt[a]![b]!, pt[a]![b + 1]!, pt[a + 1]![b + 1]!, pt[a + 1]![b]!];
          const cx = poly.reduce((s, p) => s + p[0], 0) / 4;
          const cy = poly.reduce((s, p) => s + p[1], 0) / 4;
          const el = card.cloneNode(true) as HTMLElement;
          el.style.boxShadow = "none";
          el.style.clipPath = `polygon(${poly.map((p) => `${p[0]}% ${p[1]}%`).join(",")})`;
          el.style.transformOrigin = `${cx}% ${cy}%`;
          show.insertBefore(el, flash);
          const dx = x + ((cx - 50) / 100) * w - 640;
          const dy = y + ((cy - 50) / 100) * h - 300;
          const len = Math.hypot(dx, dy) || 1;
          const speed = 500 + rand() * 700;
          made.push({
            el,
            x,
            y,
            r,
            vx: (dx / len) * speed + (rand() - 0.5) * 240,
            vy: (dy / len) * speed - rand() * 420,
            vr: (rand() - 0.5) * 760,
          });
        }
      }
    }
  };

  let callH = 0;
  let lineY: number[] = [];
  const pop = (t: number, a: number, d = 0.4) => seg(t, a, a + d, out);

  function render(t: number) {
    const before = Math.min(t, R - 0.01);
    const dt = t - R;
    const alive = t < R ? 1 : 0;

    // Hex: the call. After the impact it slides into Hex Meetings' list.
    const callX = t < R ? mix(640, 560, seg(t, 4.6, 5.3)) : mix(560, 190, seg(t, R + 0.6, R + 1.4));
    place(call, callX, 300, {
      o: pop(t, 0.3, 0.6) * (1 - seg(t, R + 1.2, R + 1.6)),
      s: mix(0.94, 1, pop(t, 0.3, 0.6)) * mix(1, 0.72, seg(t, R + 0.6, R + 1.4)),
    });
    lines.forEach((l, i) => {
      const p = seg(t, 1.2 + i * 0.6, 1.6 + i * 0.6, out);
      l.style.opacity = String(p);
      l.style.translate = `0 ${(1 - p) * 8}px`;
    });

    // Copy, paste and prompt.
    place(chat, 1030, 230, { o: pop(t, 4.6, 0.5) * alive, s: mix(0.94, 1, pop(t, 4.6, 0.5)) });
    typeIn(prompt, PROMPT, 5.0, 40, before);
    pasted.style.opacity = String(seg(before, 7.6, 7.8));
    skel.forEach((s, i) => (s.style.clipPath = `inset(0 ${100 - seg(before, 8.3 + i * 0.3, 8.7 + i * 0.3, (x) => x) * 100}% 0 0)`));
    copy.innerHTML = before >= 9.65 ? `${glyphs.copy} Copied` : `${glyphs.copy} Copy`;
    copy.style.scale = String(before > 9.6 && before < 9.72 ? 0.92 : 1);
    const cardTop = 300 - callH / 2;
    const fly = (el: HTMLElement, a: number, from: XY, to: XY, d = 0.7) => {
      const p = seg(t, a, a + d);
      const q = 1 - p;
      const x = q * q * from.x + 2 * q * p * mix(from.x, to.x, 0.5) + p * p * to.x;
      const y = q * q * from.y + 2 * q * p * (Math.min(from.y, to.y) - 140) + p * p * to.y;
      const on = t > a && t < a + d ? Math.min(1, (t - a) / 0.1, (a + d - t) / 0.12) : 0;
      place(el, x, y, { o: on, s: mix(1, 0.65, p) });
    };
    fliers.forEach((f, i) => fly(f, 6.9 + i * 0.3, { x: callX - 60, y: cardTop + lineY[i]! }, { x: 1000, y: 248 }));

    // Then again for each place, by hand.
    (Object.keys(HAND) as (keyof typeof HAND)[]).forEach((k, i) => {
      const { x, y, at } = HAND[k];
      place(hands[k], x, y, { o: pop(t, at) * alive, s: mix(0.92, 1, pop(t, at)) });
      fly(copied[i]!, at + 0.05, { x: 1080, y: 300 }, { x, y }, 0.45);
      const [el, text] = handText[k];
      typeIn(el, text, at + 0.45, 70, before);
    });
    typeIn(githubTitle, "CSV export fails on big accounts", HAND.github.at + 0.3, 50, before);

    // Meanwhile the next calls arrive.
    const shake = 4 * seg(t, 15, 19.4, (x) => x ** 1.5) + 6 * seg(t, 18.8, 19.5);
    notes.forEach((n, i) => {
      const [at, , , , x, y, r] = NOTES[i]!;
      const p = pop(t, at, 0.35);
      const dx = Math.sin(t * 37 + i * 2.1) * shake;
      const dy = Math.cos(t * 41 + i * 1.3) * shake;
      place(n, x + dx, y + dy - (1 - p) * 24, { o: p * alive, s: mix(0.88, 1, p), r: r + dx * 0.3 });
    });
    vignette.style.opacity = String(t < R ? seg(t, 13, 19.4, (x) => x) * (0.6 + 0.4 * Math.sin(t * 6) ** 2) : 0);

    // Hex Meetings lands.
    const f = dt >= 0 && dt < 0.7 ? 1 - seg(dt, 0.05, 0.7, out) : 0;
    place(flash, 640, 300, { o: f, s: mix(1.3, 0.6, f) });
    ringPaths.forEach((p, i) => {
      const k = seg(dt - i * 0.12, 0, 1.3, out);
      p.setAttribute("d", hexagon(640, 300, 60 + 1000 * k));
      p.setAttribute("stroke-width", String(3 - i));
      p.style.opacity = String(dt >= i * 0.12 ? 0.85 * (1 - k) : 0);
    });
    const landed = seg(dt, -0.05, 0.45, back);
    const up = seg(dt, 1.0, 1.8);
    place(mark, 640, mix(300, 58, up), { o: dt >= -0.05 ? Math.min(1, landed * 3) : 0, s: landed * mix(1, 0.42, up) });
    if (shards) {
      for (const s of shards) {
        if (t < R) {
          s.el.style.visibility = "hidden";
          continue;
        }
        const x = s.x + s.vx * dt;
        const y = s.y + s.vy * dt + 0.5 * 1500 * dt * dt;
        place(s.el, x, y, { o: 1 - seg(dt, 0.5, 1.5, (v) => v), r: s.r + s.vr * dt, s: mix(1, 0.85, seg(dt, 0, 1.5)) });
      }
    }

    // Every call Hex records, in one list.
    place(list, 190, 330, { o: pop(t, R + 1.3, 0.5), s: mix(0.94, 1, pop(t, R + 1.3, 0.5)) });

    // A saved report, run with your own agent.
    place(panel, 610, 330, { o: pop(t, R + 2.0, 0.5), s: mix(0.94, 1, pop(t, R + 2.0, 0.5)) });
    const shown = clamp(AGENT_ORDER.filter((_, i) => t >= AGENT_AT(i)).length - 1, 0, AGENT_ORDER.length - 1);
    const id = AGENT_ORDER[shown]!;
    if (face.dataset.id !== id) {
      face.dataset.id = id;
      face.innerHTML = `${img(marks[id], 14)} <span>${AGENT_NAMES[id]}</span>`;
    }
    const flip = t >= AGENT_AT(0) ? seg(t, AGENT_AT(shown), AGENT_AT(shown) + 0.22, out) : 1;
    face.style.translate = `0 ${(1 - flip) * 10}px`;
    face.style.opacity = String(flip);
    run.style.scale = String(dt > 4.8 && dt < 4.95 ? 0.92 : 1);
    run.style.background = dt >= 4.8 ? "#2f56d040" : "";
    run.style.color = dt >= 4.8 ? "#ededed" : "";
    const w = [
      seg(t, R + 5.2, R + 5.9, (x) => x),
      seg(t, R + 5.9, R + 6.05, (x) => x),
      ...ITEMS.map((_, i) => seg(t, R + 6.05 + i * 0.35, R + 6.35 + i * 0.35, (x) => x)),
    ];
    wipes.forEach((el, i) => el.style.setProperty("--w", String(w[i])));
    items.forEach((el, i) => {
      const c = seg(t, R + 6.05 + i * 0.35, R + 6.3 + i * 0.35, out);
      const tick = el.querySelector<HTMLElement>(".tick")!;
      tick.style.background = `rgba(47, 86, 208, ${c})`;
      tick.style.borderColor = c > 0 ? "#8fb0ff" : "#ffffff40";
      const path = tick.querySelector("path")!;
      path.style.strokeDasharray = "1";
      path.style.strokeDashoffset = String(1 - c);
      el.style.background = `rgba(47, 86, 208, ${0.14 * c})`;
      el.style.opacity = String(seg(t, R + 6.0 + i * 0.35, R + 6.12 + i * 0.35));
    });

    // You read it, then it goes where it belongs.
    const posted = t >= R + 8.45;
    place(preview, 1060, 290, { o: pop(t, R + 6.9, 0.5), s: mix(0.94, 1, pop(t, R + 6.9, 0.5)) });
    postButton.style.scale = String(dt > 8.3 && dt < 8.45 ? 0.9 : 1);
    postButtons.forEach((b) => (b.style.opacity = String(1 - seg(t, R + 8.5, R + 8.8))));
    state.textContent = posted ? "Posted" : "Ready to post";
    state.classList.toggle("posted", posted);
    goes.textContent = posted ? "Posted to Slack (#sales)" : "Goes to Slack (#sales)";
    const ok = dt > 8.45 ? Math.sin(Math.PI * clamp((dt - 8.45) / 1.2)) : 0;
    preview.style.boxShadow = `0 18px 50px #000b, 0 0 ${40 * ok}px ${ok > 0 ? "#30a46caa" : "transparent"}`;
    tiles.forEach((tile, i) => {
      const p = pop(t, R + 9.0 + i * 0.18, 0.4);
      place(tile, 950 + i * 112, 510 + (1 - p) * 14, { o: p, s: mix(0.9, 1, p) });
    });

    captions.forEach((c, i) => {
      const [a, b] = CAPTIONS[i]!;
      const o = seg(t, a, a + 0.35, out) * (1 - seg(t, b - 0.3, b));
      place(c, 640, 612 + (1 - seg(t, a, a + 0.35, out)) * 10, { o });
    });
  }

  const sound = new Sound();
  const cueList = cues(sound);

  let raf = 0;
  const play = () => {
    cancelAnimationFrame(raf);
    sound.start();
    const wait = open(true);
    controls.dataset.on = "";
    replay.hidden = true;
    const t0 = performance.now() + wait * 1000;
    let last = -1;
    const frameAt = (now: number) => {
      const t = (now - t0) / 1000;
      for (const [at, cue] of cueList) if (at > last && at <= t) cue();
      last = Math.max(t, last);
      render(clamp(t, 0, END));
      if (t < END) raf = requestAnimationFrame(frameAt);
      else replay.hidden = false;
    };
    raf = requestAnimationFrame(frameAt);
  };
  playKey.addEventListener("click", play);
  replay.addEventListener("click", () => {
    sound.stopDrone();
    play();
  });
  const showMute = () => {
    muteButton.innerHTML = sound.muted ? `${glyphs.soundOff} Sound off` : `${glyphs.soundOn} Sound on`;
    muteButton.setAttribute("aria-pressed", String(sound.muted));
  };
  showMute();
  muteButton.addEventListener("click", () => {
    sound.setMuted(!sound.muted);
    showMute();
  });
  playKey.addEventListener("pointerenter", () => sound.prepare());
  playKey.addEventListener("focus", () => sound.prepare());

  // Measure once, with the real font, and break the cards ahead of time so the impact doesn't stall.
  await document.fonts.ready;
  callH = call.offsetHeight;
  lineY = lines.map((l) => l.offsetTop + l.offsetHeight / 2);
  render(R - 0.01);
  shatter();

  const at = new URLSearchParams(location.search).get("t");
  if (at !== null) {
    open(false);
    render(Number(at));
  } else if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    open(false);
    render(END);
  } else {
    render(0);
  }
}

type XY = { x: number; y: number };

/** A hexagon with points left and right, like the site's outlines. */
function hexagon(cx: number, cy: number, r: number) {
  const h = r * 0.866;
  return `M ${cx - r} ${cy} L ${cx - r / 2} ${cy - h} H ${cx + r / 2} L ${cx + r} ${cy} L ${cx + r / 2} ${cy + h} H ${cx - r / 2} Z`;
}
