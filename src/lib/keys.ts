// Keyboard shortcuts while a video is playing. Used by whichever window has
// keyboard focus (the controls window, or the main window behind it).
import { act, player, P } from "./player.svelte";
import { settings } from "./settings.svelte";
import { pickMedia, screenshot } from "./menus";
import { closeMenu, ui } from "./ui.svelte";
import { api } from "./api";

export interface KeyHost {
  toggleQueue: () => void;
  toggleInfo: () => void;
  toggleShortcuts: () => void;
  openSettings: () => void;
  osd: (text: string) => void;
}

export const SHORTCUTS: [string, string][] = [
  ["Space / K", "Play or pause"],
  ["← / →", "Back / forward 5 seconds"],
  ["Shift + ← / →", "Back / forward 30 seconds"],
  ["J / L", "Back / forward 10 seconds"],
  ["↑ / ↓", "Volume up / down"],
  ["M", "Mute"],
  ["F or double-click", "Full screen"],
  ["Esc", "Leave full screen or picture in picture"],
  ["Backspace", "Back to the library"],
  ["Shift + N / Shift + B", "Next / previous in the queue"],
  ["0 … 9", "Jump to 0% … 90%"],
  [", / .", "Previous / next frame (paused)"],
  ["[ / ]", "Slower / faster"],
  ["=", "Normal speed"],
  ["C", "Next subtitle track"],
  ["V", "Show or hide subtitles"],
  ["Z / X", "Subtitles earlier / later"],
  ["A", "Next audio track"],
  ["S", "Screenshot"],
  ["P", "Picture in picture"],
  ["T", "Always on top"],
  ["Q", "Queue"],
  ["I", "Media info"],
  ["Ctrl + O", "Open file"],
  ["F1 or ?", "Keyboard shortcuts"],
];

export function handlePlayerKey(e: KeyboardEvent, host: KeyHost): boolean {
  const target = e.target as HTMLElement | null;
  if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) return false;
  const step = Number(settings.v["playback.seek_step"] ?? 5);
  const big = Number(settings.v["playback.seek_step_large"] ?? 30);
  const k = e.key;
  const ctrl = e.ctrlKey || e.metaKey;
  let handled = true;
  if (ctrl) {
    if (k === "o" || k === "O") pickMedia("replace");
    else if (k === ",") host.openSettings();
    else handled = false;
  } else if (e.altKey) {
    if (k === "ArrowLeft") act.back();
    else if (k === "Enter") act.fullscreen();
    else handled = false;
  } else {
    switch (k) {
      case " ":
      case "k":
      case "K":
      case "MediaPlayPause":
        act.togglePause();
        break;
      case "ArrowLeft":
        act.seekBy(e.shiftKey ? -big : -step);
        host.osd(`−${e.shiftKey ? big : step}s`);
        break;
      case "ArrowRight":
        act.seekBy(e.shiftKey ? big : step);
        host.osd(`+${e.shiftKey ? big : step}s`);
        break;
      case "j":
      case "J":
        act.seekBy(-10);
        host.osd("−10s");
        break;
      case "l":
      case "L":
        if (e.shiftKey) act.abLoop();
        else {
          act.seekBy(10);
          host.osd("+10s");
        }
        break;
      case "ArrowUp":
        act.volumeBy(5);
        break;
      case "ArrowDown":
        act.volumeBy(-5);
        break;
      case "m":
      case "M":
        act.toggleMute();
        break;
      case "f":
      case "F":
        act.fullscreen();
        break;
      case "Escape":
        if (ui.menu) closeMenu();
        else if (player.fullscreen) act.fullscreen(false);
        else if (player.pip) act.pip(false);
        else handled = false;
        break;
      case "Backspace":
        act.back();
        break;
      case "N":
        act.next();
        break;
      case "B":
        act.prev();
        break;
      case "MediaTrackNext":
        act.next();
        break;
      case "MediaTrackPrevious":
        act.prev();
        break;
      case ",":
        act.frameStep(true);
        break;
      case ".":
        act.frameStep(false);
        break;
      case "[":
        act.speedBy(-1);
        break;
      case "]":
        act.speedBy(1);
        break;
      case "=":
      case "Backquote":
        act.setSpeed(1);
        host.osd("1×");
        break;
      case "c":
      case "C":
        act.cycleSub();
        break;
      case "v":
      case "V":
        act.toggleSubs();
        break;
      case "z":
      case "Z":
        act.subDelayBy(-0.1);
        break;
      case "x":
      case "X":
        act.subDelayBy(0.1);
        break;
      case "a":
      case "A":
        act.cycleAudio();
        break;
      case "s":
      case "S":
        screenshot();
        break;
      case "p":
      case "P":
        act.pip();
        break;
      case "t":
      case "T":
        act.onTop(!player.onTop);
        break;
      case "q":
      case "Q":
        host.toggleQueue();
        break;
      case "i":
      case "I":
        host.toggleInfo();
        break;
      case "F1":
      case "?":
        host.toggleShortcuts();
        break;
      case "F11":
        act.fullscreen();
        break;
      default:
        if (/^[0-9]$/.test(k) && P.duration > 0) act.seekPercent(Number(k) * 10);
        else handled = false;
    }
  }
  if (handled) {
    e.preventDefault();
    e.stopPropagation();
  }
  return handled;
}

export { api };
