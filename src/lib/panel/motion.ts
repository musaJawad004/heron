// One place for the panel's motion budget: 150 ms, or nothing when the OS
// asks for reduced motion.
export const reducedMotion =
  typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Duration for `slide` / `flip` on the panel. */
export const duration = reducedMotion ? 0 : 150;
