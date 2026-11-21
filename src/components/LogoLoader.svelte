<script lang="ts">
  // Loading indicator: the MEDORE mark breathing gently while a silver light
  // sweeps across it. Still (no motion) when animations are reduced.
  let { size = 64 }: { size?: number } = $props();
  const uid = "ll" + Math.random().toString(36).slice(2, 8);
</script>

<svg class="loader" width={size} height={size * 0.8} viewBox="120 200 750 600" role="img" aria-label="Loading">
  <defs>
    <linearGradient id="{uid}f" gradientUnits="userSpaceOnUse" x1="160" y1="230" x2="660" y2="520"><stop offset="0" stop-color="#fff" /><stop offset=".55" stop-color="#F1F1F3" /><stop offset="1" stop-color="#C9C9CE" /></linearGradient>
    <linearGradient id="{uid}b" gradientUnits="userSpaceOnUse" x1="520" y1="500" x2="830" y2="240"><stop offset="0" stop-color="#5E5E64" /><stop offset=".42" stop-color="#B9B9BF" /><stop offset="1" stop-color="#F6F6F8" /></linearGradient>
    <linearGradient id="{uid}l" gradientUnits="userSpaceOnUse" x1="250" y1="395" x2="170" y2="770"><stop offset="0" stop-color="#606066" /><stop offset=".3" stop-color="#C6C6CB" /><stop offset="1" stop-color="#F4F4F6" /></linearGradient>
    <linearGradient id="{uid}s" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#fff" stop-opacity="0" /><stop offset=".5" stop-color="#fff" stop-opacity=".85" /><stop offset="1" stop-color="#fff" stop-opacity="0" /></linearGradient>
    <clipPath id="{uid}c">
      <path d="M505 372 C620 300 700 228 760 228 C815 228 845 262 845 321 L845 695 A75 75 0 0 1 695 695 L695 427 L505 527 Z" />
      <path d="M150 480 L150 321 C150 262 180 228 236 228 C275 228 300 236 330 252 L669 441 L505 527 L305 418 L305 480 Z" />
      <rect x="143" y="395" width="169" height="382" rx="84.5" />
      <path d="M436 574 L436 742 L600 658 Z" />
    </clipPath>
  </defs>
  <g class="mark">
    <path d="M505 372 C620 300 700 228 760 228 C815 228 845 262 845 321 L845 695 A75 75 0 0 1 695 695 L695 427 L505 527 Z" fill="url(#{uid}b)" stroke="url(#{uid}b)" stroke-width="14" stroke-linejoin="round" />
    <path d="M150 480 L150 321 C150 262 180 228 236 228 C275 228 300 236 330 252 L669 441 L505 527 L305 418 L305 480 Z" fill="url(#{uid}f)" stroke="url(#{uid}f)" stroke-width="14" stroke-linejoin="round" />
    <rect x="143" y="395" width="169" height="382" rx="84.5" fill="url(#{uid}l)" />
    <path class="tri" d="M452 590 L452 726 L582 658 Z" fill="#fff" stroke="#fff" stroke-width="34" stroke-linejoin="round" />
    <g clip-path="url(#{uid}c)"><rect class="shine" x="80" y="180" width="240" height="660" fill="url(#{uid}s)" transform="skewX(-18)" /></g>
  </g>
</svg>

<style>
  .loader {
    display: block;
    overflow: visible;
    filter: drop-shadow(0 4px 18px rgba(0, 0, 0, 0.6));
  }
  .mark {
    transform-box: fill-box;
    transform-origin: center;
    animation: breathe 1.6s ease-in-out infinite;
  }
  .tri {
    transform-box: fill-box;
    transform-origin: center;
    animation: pulse 1.6s ease-in-out infinite;
  }
  .shine {
    animation: sweep 1.6s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }
  @keyframes breathe {
    0%, 100% { opacity: 0.82; transform: scale(0.97); }
    50% { opacity: 1; transform: scale(1); }
  }
  @keyframes pulse {
    0%, 100% { transform: scale(0.92); }
    50% { transform: scale(1.06); }
  }
  @keyframes sweep {
    0% { transform: translateX(-700px) skewX(-18deg); opacity: 0; }
    25% { opacity: 1; }
    70% { opacity: 1; }
    100% { transform: translateX(700px) skewX(-18deg); opacity: 0; }
  }
  :global(:root.reduce-motion) .mark,
  :global(:root.reduce-motion) .tri {
    animation: none;
  }
  :global(:root.reduce-motion) .shine {
    display: none;
  }
  @media (prefers-reduced-motion: reduce) {
    :global(:root:not(.motion-on)) .mark,
    :global(:root:not(.motion-on)) .tri {
      animation: none;
    }
    :global(:root:not(.motion-on)) .shine {
      display: none;
    }
  }
</style>
