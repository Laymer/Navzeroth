// Original decorative backdrop: a citadel on a ridge above a pine treeline, layered mist,
// an indigo glow and the project site's left-to-right scrim. Inline SVG, so it needs no
// img-src and no data: URI, and it is not an asset a third party owns. It is purely
// presentational: aria-hidden, never focusable, ignored by assistive technology.

interface Tree {
  x: number;
  height: number;
}

// Deterministic spacing and heights: no randomness, so renders and screenshots are stable.
function treeline(base: number, step: number, offset: number, min: number, spread: number): string {
  const trees: Tree[] = [];
  for (let index = 0; index * step - offset < 1700; index++) {
    trees.push({ x: index * step - offset + ((index * 37) % 19), height: min + ((index * 53) % spread) });
  }
  const parts = trees.map(({ x, height }) => {
    const half = height * 0.22;
    return `L${x - half} ${base} L${x - half * 0.45} ${base - height * 0.45} L${x - half * 0.8} ${base - height * 0.45} L${x} ${base - height} L${x + half * 0.8} ${base - height * 0.45} L${x + half * 0.45} ${base - height * 0.45} L${x + half} ${base}`;
  });
  return `M-100 900 L-100 ${base} ${parts.join(" ")} L1700 ${base} L1700 900Z`;
}

const FAR_TREES = treeline(742, 34, 60, 46, 31);
const NEAR_TREES = treeline(850, 46, 20, 78, 52);

const EMBERS = [
  { x: 1010, y: 640, delay: "0s", length: "9s" },
  { x: 1080, y: 690, delay: "2.3s", length: "11s" },
  { x: 1160, y: 660, delay: "5.1s", length: "10s" },
  { x: 1240, y: 700, delay: "1.2s", length: "12s" },
  { x: 1310, y: 650, delay: "6.4s", length: "9.5s" },
  { x: 1390, y: 690, delay: "3.7s", length: "13s" },
  { x: 940, y: 720, delay: "7.9s", length: "10.5s" },
  { x: 1450, y: 730, delay: "4.4s", length: "11.5s" },
] as const;

const WINDOWS: readonly (readonly [number, number])[] = [
  [1118, 520], [1138, 520], [1214, 498], [1232, 498], [1265, 540], [1180, 560], [1330, 548], [1108, 468], [1250, 440],
];

export function Backdrop() {
  return (
    <div className="backdrop" aria-hidden="true">
      <svg viewBox="0 0 1600 900" preserveAspectRatio="xMidYMax slice" focusable="false">
        <defs>
          <linearGradient id="anav-sky" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#0d0e1a" />
            <stop offset="0.55" stopColor="#1b1d3b" />
            <stop offset="1" stopColor="#161826" />
          </linearGradient>
          <radialGradient id="anav-glow" cx="1150" cy="360" r="620" gradientUnits="userSpaceOnUse">
            <stop offset="0" stopColor="#5a4cc4" stopOpacity="0.55" />
            <stop offset="0.45" stopColor="#3a3590" stopOpacity="0.22" />
            <stop offset="1" stopColor="#161826" stopOpacity="0" />
          </radialGradient>
          <linearGradient id="anav-mist" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#44505f" stopOpacity="0" />
            <stop offset="0.5" stopColor="#44505f" stopOpacity="0.42" />
            <stop offset="1" stopColor="#44505f" stopOpacity="0" />
          </linearGradient>
          <linearGradient id="anav-scrim" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0" stopColor="#161826" stopOpacity="0.82" />
            <stop offset="0.36" stopColor="#161826" stopOpacity="0.58" />
            <stop offset="0.72" stopColor="#161826" stopOpacity="0.1" />
            <stop offset="1" stopColor="#161826" stopOpacity="0" />
          </linearGradient>
        </defs>
        <rect width="1600" height="900" fill="url(#anav-sky)" />
        <rect width="1600" height="900" fill="url(#anav-glow)" />
        {/* Distant ridge. */}
        <path d="M0 640 C140 590 260 630 400 596 S660 540 820 580 S1100 520 1280 560 S1520 560 1600 530 L1600 900 L0 900Z" fill="#1c2040" />
        <rect y="560" width="1600" height="110" fill="url(#anav-mist)" />
        {/* Citadel on the ridge: curtain wall, keep, towers with pointed roofs, a gate arch. */}
        <g fill="#10111e">
          <path d="M980 612 L980 520 L1000 520 L1000 506 L1016 506 L1016 520 L1040 520 L1040 506 L1056 506 L1056 520 L1420 520 L1420 612Z" />
          <rect x="1090" y="440" width="190" height="130" />
          <path d="M1090 440 L1090 424 L1106 424 L1106 440 M1126 440 L1126 424 L1142 424 L1142 440 M1162 440 L1162 424 L1178 424 L1178 440 M1198 440 L1198 424 L1214 424 L1214 440 M1234 440 L1234 424 L1250 424 L1250 440 M1266 440 L1266 424 L1280 424 L1280 440Z" />
          <rect x="1196" y="360" width="46" height="90" />
          <path d="M1186 362 L1219 270 L1252 362Z" />
          <rect x="1062" y="452" width="40" height="70" />
          <path d="M1054 454 L1082 388 L1110 454Z" />
          <rect x="1288" y="470" width="44" height="60" />
          <path d="M1280 472 L1310 410 L1340 472Z" />
          <rect x="1366" y="486" width="36" height="40" />
          <path d="M1360 488 L1384 440 L1408 488Z" />
          <path d="M1160 612 L1160 566 Q1186 534 1212 566 L1212 612Z" fill="#0a0b14" />
          <path d="M1219 270 L1219 238 L1244 248 L1219 256Z" fill="#7a2f1d" />
        </g>
        <g fill="#e6c66a" opacity="0.55">
          {WINDOWS.map(([x, y]) => (
            <rect key={`${x}-${y}`} x={x} y={y} width="7" height="12" rx="2" />
          ))}
        </g>
        {/* Far pine treeline, mist, near ridge and near treeline. */}
        <path d={FAR_TREES} fill="#13152a" />
        <rect y="640" width="1600" height="150" fill="url(#anav-mist)" opacity="0.8" />
        <path d="M0 780 C220 740 420 790 640 764 S1040 730 1260 770 S1500 780 1600 760 L1600 900 L0 900Z" fill="#101223" />
        <path d={NEAR_TREES} fill="#0b0c17" />
        <g className="backdrop-embers">
          {EMBERS.map(({ x, y, delay, length }) => (
            <circle key={x} className="backdrop-ember" cx={x} cy={y} r="2.4" fill="#e08a4a" style={{ animationDelay: delay, animationDuration: length }} />
          ))}
        </g>
        {/* The site's left-to-right scrim keeps text legible over the art. */}
        <rect width="1600" height="900" fill="url(#anav-scrim)" />
      </svg>
    </div>
  );
}
