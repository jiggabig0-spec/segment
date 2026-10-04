// The Segments mark: a ring cut into equal segments, with one pulled out in the accent colour.
// Source of truth for the PNG exports in /mnt/project-files/platform/logo.
const SEGMENTS = [
  { d: "M544.74 100.29 A372 372 0 0 1 852.18 277.79 L662.90 393.79 A150 150 0 0 0 538.93 322.21 Z", accent: true },
  { d: "M838.92 334.50 A372 372 0 0 1 838.92 689.50 L643.82 583.57 A150 150 0 0 0 643.82 440.43 Z", accent: false },
  { d: "M829.18 706.37 A372 372 0 0 1 521.74 883.87 L515.93 661.95 A150 150 0 0 0 639.90 590.37 Z", accent: false },
  { d: "M502.26 883.87 A372 372 0 0 1 194.82 706.37 L384.10 590.37 A150 150 0 0 0 508.07 661.95 Z", accent: false },
  { d: "M185.08 689.50 A372 372 0 0 1 185.08 334.50 L380.18 440.43 A150 150 0 0 0 380.18 583.57 Z", accent: false },
  { d: "M194.82 317.63 A372 372 0 0 1 502.26 140.13 L508.07 362.05 A150 150 0 0 0 384.10 433.63 Z", accent: false },
];

export function LogoMark({ size = 24 }: { size?: number }) {
  return (
    <svg viewBox="0 0 1024 1024" width={size} height={size} aria-hidden="true" className="logo-mark">
      {SEGMENTS.map((s, i) => (
        <path key={i} d={s.d} fill={s.accent ? "var(--accent)" : "currentColor"} />
      ))}
    </svg>
  );
}
