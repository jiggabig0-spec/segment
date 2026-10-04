"use client";

import { useEffect, useRef, useState } from "react";
import { formatAmount } from "@/lib/amounts";

const DURATION_MS = 700;
const easeOut = (t: number) => 1 - Math.pow(1 - t, 3);

function prefersReducedMotion() {
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
}

/**
 * A raw token amount that counts smoothly to its new value when it changes, and briefly
 * highlights. The final frame always shows the exact value.
 */
export function AnimatedNumber({ value, decimals, maxFraction }: { value: bigint; decimals: number; maxFraction?: number }) {
  const [shown, setShown] = useState(value);
  const [flash, setFlash] = useState(false);
  const from = useRef(value);
  const first = useRef(true);

  useEffect(() => {
    const start = from.current;
    from.current = value;
    if (first.current || start === value || prefersReducedMotion()) {
      first.current = false;
      setShown(value);
      return;
    }
    setFlash(true);
    const t0 = performance.now();
    let raf = 0;
    const tick = (now: number) => {
      const t = Math.min(1, (now - t0) / DURATION_MS);
      // Interpolate in bigint space, scaled by 1e6, so large 8-decimal amounts stay exact at t = 1.
      const k = BigInt(Math.round(easeOut(t) * 1e6));
      setShown(start + ((value - start) * k) / BigInt(1e6));
      if (t < 1) raf = requestAnimationFrame(tick);
      else setTimeout(() => setFlash(false), 300);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [value]);

  return <span className={flash ? "num-anim flash" : "num-anim"}>{formatAmount(shown, decimals, maxFraction)}</span>;
}
