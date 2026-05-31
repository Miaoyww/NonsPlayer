/**
 * AudioEffectManager — manages EQ, filters, and spectrum analysis.
 *
 * Audio chain:
 *   Input → HighPass → LowPass → EQ[0] → EQ[1] → ... → EQ[9] → Analyser → Output
 */

export class AudioEffectManager {
  private audioCtx: AudioContext;

  private analyserNode: AnalyserNode | null = null;
  private filters: BiquadFilterNode[] = [];
  private highPassFilter: BiquadFilterNode | null = null;
  private lowPassFilter: BiquadFilterNode | null = null;

  /** EMA-smoothed low-frequency volume for visual effects */
  private smoothedLowFreqVolume = 0;

  /** 10-band EQ center frequencies */
  private readonly eqFrequencies = [
    32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
  ];

  constructor(context: AudioContext) {
    this.audioCtx = context;
    this.initNodes();
  }

  private initNodes() {
    // Analyser for spectrum visualization
    this.analyserNode = this.audioCtx.createAnalyser();
    this.analyserNode.fftSize = 512;

    // 10-band peaking EQ
    this.filters = this.eqFrequencies.map((freq) => {
      const filter = this.audioCtx.createBiquadFilter();
      filter.type = "peaking";
      filter.frequency.value = freq;
      filter.Q.value = 1;
      filter.gain.value = 0;
      return filter;
    });

    // Highpass filter (for crossfade bass-swap)
    this.highPassFilter = this.audioCtx.createBiquadFilter();
    this.highPassFilter.type = "highpass";
    this.highPassFilter.frequency.value = 0; // off by default
    this.highPassFilter.Q.value = 0.707;

    // Lowpass filter
    this.lowPassFilter = this.audioCtx.createBiquadFilter();
    this.lowPassFilter.type = "lowpass";
    this.lowPassFilter.frequency.value = 22000; // off by default
    this.lowPassFilter.Q.value = 0.707;
  }

  /**
   * Connect the effect chain into the audio graph.
   * Chain: Input → HighPass → LowPass → EQ[0..9] → Analyser → returns last node
   */
  public connect(inputNode: AudioNode): AudioNode {
    let currentNode = inputNode;

    if (this.highPassFilter) {
      currentNode.connect(this.highPassFilter);
      currentNode = this.highPassFilter;
    }
    if (this.lowPassFilter) {
      currentNode.connect(this.lowPassFilter);
      currentNode = this.lowPassFilter;
    }

    for (const filter of this.filters) {
      currentNode.connect(filter);
      currentNode = filter;
    }

    if (this.analyserNode) {
      currentNode.connect(this.analyserNode);
      currentNode = this.analyserNode;
    }

    return currentNode;
  }

  // ── Highpass filter ──

  setHighPassFilter(frequency: number, rampTime = 0) {
    if (!this.highPassFilter) return;
    const now = this.audioCtx.currentTime;
    this.highPassFilter.frequency.cancelScheduledValues(now);

    if (frequency <= 0) {
      this.highPassFilter.type = "allpass";
      this.highPassFilter.frequency.setValueAtTime(10, now);
      return;
    }

    this.highPassFilter.type = "highpass";
    const freq = Math.max(10, Math.min(22000, frequency));
    if (rampTime > 0) {
      this.highPassFilter.frequency.exponentialRampToValueAtTime(freq, now + rampTime);
    } else {
      this.highPassFilter.frequency.setValueAtTime(freq, now);
    }
  }

  setHighPassQ(q: number) {
    if (!this.highPassFilter) return;
    const safeQ = Math.max(0.1, Math.min(10, q));
    this.highPassFilter.Q.cancelScheduledValues(this.audioCtx.currentTime);
    this.highPassFilter.Q.setValueAtTime(safeQ, this.audioCtx.currentTime);
  }

  setHighPassFilterAt(frequency: number, when: number) {
    if (!this.highPassFilter) return;
    const time = Math.max(when, this.audioCtx.currentTime);
    const freq = frequency <= 0 ? 10 : Math.max(10, Math.min(22000, frequency));
    this.highPassFilter.type = "highpass";
    this.highPassFilter.frequency.cancelScheduledValues(time);
    this.highPassFilter.frequency.setValueAtTime(freq, time);
  }

  rampHighPassFilterToAt(frequency: number, when: number) {
    if (!this.highPassFilter) return;
    const time = Math.max(when, this.audioCtx.currentTime);
    const freq = frequency <= 0 ? 10 : Math.max(10, Math.min(22000, frequency));
    this.highPassFilter.type = "highpass";
    this.highPassFilter.frequency.exponentialRampToValueAtTime(freq, time);
  }

  setHighPassQAt(q: number, when: number) {
    if (!this.highPassFilter) return;
    const time = Math.max(when, this.audioCtx.currentTime);
    const safeQ = Math.max(0.1, Math.min(10, q));
    this.highPassFilter.Q.cancelScheduledValues(time);
    this.highPassFilter.Q.setValueAtTime(safeQ, time);
  }

  // ── Lowpass filter ──

  setLowPassFilter(frequency: number, rampTime = 0) {
    if (!this.lowPassFilter) return;
    const now = this.audioCtx.currentTime;
    this.lowPassFilter.frequency.cancelScheduledValues(now);

    if (frequency <= 0 || frequency >= 22000) {
      this.lowPassFilter.type = "allpass";
      this.lowPassFilter.frequency.setValueAtTime(22000, now);
      return;
    }

    this.lowPassFilter.type = "lowpass";
    const freq = Math.max(10, Math.min(22000, frequency));
    if (rampTime > 0) {
      this.lowPassFilter.frequency.exponentialRampToValueAtTime(freq, now + rampTime);
    } else {
      this.lowPassFilter.frequency.setValueAtTime(freq, now);
    }
  }

  setLowPassQ(q: number) {
    if (!this.lowPassFilter) return;
    const safeQ = Math.max(0.1, Math.min(10, q));
    this.lowPassFilter.Q.cancelScheduledValues(this.audioCtx.currentTime);
    this.lowPassFilter.Q.setValueAtTime(safeQ, this.audioCtx.currentTime);
  }

  setLowPassFilterAt(frequency: number, when: number) {
    if (!this.lowPassFilter) return;
    const time = Math.max(when, this.audioCtx.currentTime);
    const freq = frequency <= 0 || frequency >= 22000
      ? 22000
      : Math.max(10, Math.min(22000, frequency));
    this.lowPassFilter.type = "lowpass";
    this.lowPassFilter.frequency.cancelScheduledValues(time);
    this.lowPassFilter.frequency.setValueAtTime(freq, time);
  }

  rampLowPassFilterToAt(frequency: number, when: number) {
    if (!this.lowPassFilter) return;
    const time = Math.max(when, this.audioCtx.currentTime);
    const freq = frequency <= 0 || frequency >= 22000
      ? 22000
      : Math.max(10, Math.min(22000, frequency));
    this.lowPassFilter.type = "lowpass";
    this.lowPassFilter.frequency.exponentialRampToValueAtTime(freq, time);
  }

  setLowPassQAt(q: number, when: number) {
    if (!this.lowPassFilter) return;
    const time = Math.max(when, this.audioCtx.currentTime);
    const safeQ = Math.max(0.1, Math.min(10, q));
    this.lowPassFilter.Q.cancelScheduledValues(time);
    this.lowPassFilter.Q.setValueAtTime(safeQ, time);
  }

  // ── EQ ──

  setFilterGain(index: number, value: number) {
    if (this.filters[index]) {
      this.filters[index].gain.value = value;
    }
  }

  getFilterGains(): number[] {
    return this.filters.map((f) => f.gain.value);
  }

  // ── Spectrum ──

  getFrequencyData(): Uint8Array {
    if (!this.analyserNode) return new Uint8Array(0);
    const data = new Uint8Array(this.analyserNode.frequencyBinCount);
    this.analyserNode.getByteFrequencyData(data);
    return data;
  }

  /**
   * Low-frequency energy (0-280Hz), EMA-smoothed, range 0-1.
   * Useful for driving background animations / visual effects.
   */
  getLowFrequencyVolume(): number {
    if (!this.analyserNode) return 0;

    const data = new Uint8Array(this.analyserNode.frequencyBinCount);
    this.analyserNode.getByteFrequencyData(data);

    // First ~3 bins ≈ 0-280Hz at 48kHz with 512 FFT
    const lowBins = data.slice(0, 3);
    const sum = lowBins.reduce((a, v) => a + v, 0);
    const avg = sum / lowBins.length;

    const threshold = 180;
    const maxValue = 255;
    const normalized = Math.max(0, (avg - threshold) / (maxValue - threshold));
    const rawValue = normalized ** 2;

    const smoothFactor = 0.28;
    this.smoothedLowFreqVolume =
      this.smoothedLowFreqVolume + smoothFactor * (rawValue - this.smoothedLowFreqVolume);

    return this.smoothedLowFreqVolume;
  }

  // ── Cleanup ──

  disconnect() {
    this.filters.forEach((f) => f.disconnect());
    this.highPassFilter?.disconnect();
    this.lowPassFilter?.disconnect();
    this.analyserNode?.disconnect();
  }
}
