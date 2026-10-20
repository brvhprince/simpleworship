/*
 * The website's moving parts:
 *
 * - the hero's live screen, which answers the app's own keys;
 * - the Listen demo, which plays as it comes into view;
 * - the feature tabs;
 * - download links for the newest release, for the visitor's system.
 *
 * Everything shown is public domain: John Newton's "Amazing Grace" and the
 * King James Version.
 */

const REPO = "brvhprince/simpleworship";

// ── The live screen ──────────────────────────────────────────────────────

const SLIDES = [
  {
    label: "Verse 1",
    text: "Amazing grace! how sweet the sound,\nThat saved a wretch like me!\nI once was lost, but now am found,\nWas blind, but now I see.",
  },
  {
    label: "Verse 2",
    text: "’Twas grace that taught my heart to fear,\nAnd grace my fears relieved;\nHow precious did that grace appear\nThe hour I first believed!",
  },
  {
    label: "Verse 3",
    text: "Through many dangers, toils and snares,\nI have already come;\n’Tis grace hath brought me safe thus far,\nAnd grace will lead me home.",
  },
  {
    label: "Verse 4",
    text: "When we’ve been there ten thousand years,\nBright shining as the sun,\nWe’ve no less days to sing God’s praise\nThan when we first begun.",
  },
];

const screen = document.getElementById("screen");
const slide = document.getElementById("slide");
const strip = document.getElementById("strip");
const flags = document.getElementById("flags");
let current = 0;
/** "black", "logo", "clear" or null. */
let state = null;

for (const [index, { label, text }] of SLIDES.entries()) {
  const item = document.createElement("li");
  const button = document.createElement("button");
  button.type = "button";
  const name = document.createElement("b");
  name.textContent = label;
  button.append(name, text.split("\n")[0]);
  button.addEventListener("click", () => show(index));
  item.append(button);
  strip.append(item);
}

/** Sends a slide live, with the app's fade. */
function show(index) {
  current = (index + SLIDES.length) % SLIDES.length;
  state = null;
  slide.classList.add("fading");
  setTimeout(() => {
    slide.textContent = SLIDES[current].text;
    slide.classList.remove("fading");
  }, 220);
  render();
}

/** Black, logo and clear text toggle, as their keys do in the app. */
function toggle(flag) {
  state = state === flag ? null : flag;
  render();
}

function render() {
  screen.classList.toggle("black", state === "black");
  screen.classList.toggle("logo", state === "logo");
  screen.classList.toggle("clear", state === "clear");
  flags.textContent = { black: "BLACK", logo: "LOGO", clear: "TEXT CLEARED" }[state] ?? "";
  for (const [index, item] of [...strip.children].entries()) {
    item.firstChild.setAttribute("aria-current", String(index === current));
  }
}

slide.textContent = SLIDES[0].text;
render();
screen.addEventListener("click", () => show(current + 1));

// The keys only work while the screen is in view, so the page still scrolls.
let screenInView = false;
new IntersectionObserver(
  ([entry]) => {
    screenInView = entry.isIntersecting;
  },
  { threshold: 0.4 },
).observe(screen);

document.addEventListener("keydown", (event) => {
  if (!screenInView || event.metaKey || event.ctrlKey || event.altKey) return;
  const handled = {
    ArrowRight: () => show(current + 1),
    ArrowDown: () => show(current + 1),
    " ": () => show(current + 1),
    PageDown: () => show(current + 1),
    ArrowLeft: () => show(current - 1),
    ArrowUp: () => show(current - 1),
    PageUp: () => show(current - 1),
    b: () => toggle("black"),
    l: () => toggle("logo"),
    c: () => toggle("clear"),
  }[event.key.length === 1 ? event.key.toLowerCase() : event.key];
  if (handled) {
    event.preventDefault();
    handled();
  }
});

// ── Listen ───────────────────────────────────────────────────────────────

/** What the preacher says, the reference in it, and the passage (KJV). */
const SERMON = [
  {
    said: "…so this morning, turn with me to John chapter three, verse sixteen.",
    heard: "John chapter three, verse sixteen",
    reference: "John 3:16",
    verse:
      "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.",
  },
  {
    said: "And keep a finger in Romans eight, verses twenty-eight to thirty.",
    heard: "Romans eight, verses twenty-eight to thirty",
    reference: "Romans 8:28–30",
    verse:
      "And we know that all things work together for good to them that love God, to them who are the called according to his purpose.",
  },
  {
    said: "Verse thirty-one says: what shall we then say to these things?",
    heard: "Verse thirty-one",
    reference: "Romans 8:31",
    verse: "What shall we then say to these things? If God be for us, who can be against us?",
  },
];

const demo = document.getElementById("listen-demo");
const transcript = document.getElementById("transcript");
const chips = document.getElementById("chips");
const verse = document.getElementById("verse");
const verseRef = document.getElementById("verse-ref");
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
let playing = false;
/** A passage is on the little screen (so new chips leave it there). */
let showing = false;

const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Shows a heard passage on the little screen, as a click does in the app. */
function preview(line) {
  showing = true;
  verse.classList.remove("waiting");
  verse.textContent = line.verse;
  verseRef.textContent = `${line.reference} (KJV)`;
}

function addChip(line) {
  const chip = document.createElement("button");
  chip.type = "button";
  chip.className = "chip";
  chip.textContent = line.reference;
  chip.title = `Heard: “${line.heard}”`;
  chip.addEventListener("click", () => preview(line));
  chips.prepend(chip);
  if (!showing) {
    verse.classList.add("waiting");
    verse.textContent = `Click ${line.reference} to show it here.`;
  }
}

/** Writes out what is said, marking the reference once it is heard. */
async function say(line) {
  const start = line.said.indexOf(line.heard);
  const end = start + line.heard.length;
  if (reduced) {
    transcript.innerHTML = "";
    transcript.append(line.said.slice(0, start), mark(line.heard), line.said.slice(end));
    return;
  }
  for (let at = 1; at <= line.said.length; at += 1) {
    transcript.textContent = line.said.slice(0, at);
    if (at === end) await wait(260);
    await wait(at < start ? 32 : 48);
  }
  transcript.innerHTML = "";
  transcript.append(line.said.slice(0, start), mark(line.heard), line.said.slice(end));
}

function mark(text) {
  const marked = document.createElement("mark");
  marked.textContent = text;
  return marked;
}

async function play() {
  if (playing) return;
  playing = true;
  demo.classList.remove("done");
  showing = false;
  chips.replaceChildren();
  verse.classList.add("waiting");
  verse.textContent = "Listening for scripture…";
  verseRef.textContent = "";
  for (const line of SERMON) {
    demo.classList.add("talking");
    await say(line);
    demo.classList.remove("talking");
    addChip(line);
    await wait(reduced ? 0 : 1600);
  }
  demo.classList.add("done");
  playing = false;
}

new IntersectionObserver(
  ([entry], observer) => {
    if (entry.isIntersecting) {
      observer.disconnect();
      void play();
    }
  },
  { threshold: 0.5 },
).observe(demo);
document.getElementById("replay").addEventListener("click", () => void play());

// ── Feature tabs ─────────────────────────────────────────────────────────

const tabs = [...document.querySelectorAll('[role="tab"]')];

function choose(tab) {
  for (const other of tabs) {
    const chosen = other === tab;
    other.setAttribute("aria-selected", String(chosen));
    other.tabIndex = chosen ? 0 : -1;
    document.getElementById(other.getAttribute("aria-controls")).hidden = !chosen;
  }
}

for (const tab of tabs) {
  tab.tabIndex = tab.getAttribute("aria-selected") === "true" ? 0 : -1;
  tab.addEventListener("click", () => choose(tab));
  tab.addEventListener("keydown", (event) => {
    const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
    if (!step) return;
    const next = tabs[(tabs.indexOf(tab) + step + tabs.length) % tabs.length];
    choose(next);
    next.focus();
  });
}

// ── Downloads ────────────────────────────────────────────────────────────

/** The visitor's system, as well as the browser can tell. */
function system() {
  const platform = (navigator.userAgentData?.platform ?? navigator.platform ?? "").toLowerCase();
  const agent = navigator.userAgent.toLowerCase();
  if (/android|iphone|ipad/.test(agent)) return null;
  if (platform.includes("mac") || agent.includes("mac os")) return "mac";
  if (platform.includes("win") || agent.includes("windows")) return "windows";
  if (platform.includes("linux") || agent.includes("linux")) return "linux";
  return null;
}

const NAMES = { mac: "Mac", windows: "Windows", linux: "Linux" };

async function downloads() {
  const yours = system();
  if (yours) {
    document.querySelector(`[data-system="${yours}"]`)?.classList.add("yours");
    document.getElementById("hero-download-label").textContent = `Download for ${NAMES[yours]}`;
  }

  let release;
  try {
    const response = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`);
    if (!response.ok) return;
    release = await response.json();
  } catch {
    // Offline or rate-limited: the links still go to the releases page.
    return;
  }

  const version = release.tag_name.replace(/^v/, "");
  const date = new Date(release.published_at).toLocaleDateString("en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  document.getElementById("version").textContent = `Version ${version}`;
  document.getElementById("download-version").textContent =
    `Version ${version}, released ${date}. Free for Mac, Windows and Linux.`;

  for (const link of document.querySelectorAll("[data-asset]")) {
    const asset = release.assets.find((file) => file.name.endsWith(link.dataset.asset));
    if (asset) link.href = asset.browser_download_url;
  }
  if (yours) {
    const main = document.querySelector(`[data-system="${yours}"] .button`);
    const hero = document.getElementById("hero-download");
    hero.href = main.href;
  }
}

void downloads();
