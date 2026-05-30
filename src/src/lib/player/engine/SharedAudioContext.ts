/**
 * Shared AudioContext — global singleton used by all audio engines.
 * Provides a master DynamicsCompressor as a final limiter before output.
 *
 * Audio graph:
 *   Engine A → masterInput → Compressor → destination
 *   Engine B → masterInput ↗
 */

let sharedContext: AudioContext | null = null;
let masterInput: GainNode | null = null;
let masterLimiter: DynamicsCompressorNode | null = null;

/**
 * Get (or create) the shared AudioContext instance.
 */
export const getSharedAudioContext = (): AudioContext => {
  if (!sharedContext) {
    const AudioContextClass =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    sharedContext = new AudioContextClass({ latencyHint: "playback" });
  }
  return sharedContext;
};

/**
 * Get (or create) the master input GainNode.
 * All engines connect their output to this node.
 */
export const getSharedMasterInput = (): GainNode => {
  const ctx = getSharedAudioContext();
  if (!masterInput) {
    masterInput = ctx.createGain();
    masterLimiter = ctx.createDynamicsCompressor();

    // Master limiter: prevents clipping, tight ratio
    masterLimiter.threshold.value = -1;
    masterLimiter.knee.value = 0;
    masterLimiter.ratio.value = 20;
    masterLimiter.attack.value = 0.003;
    masterLimiter.release.value = 0.25;

    masterInput.connect(masterLimiter);
    masterLimiter.connect(ctx.destination);
  }
  return masterInput;
};

/**
 * Get the master limiter node (for inspection / visualization).
 */
export const getSharedMasterLimiter = (): DynamicsCompressorNode | null => {
  return masterLimiter;
};
