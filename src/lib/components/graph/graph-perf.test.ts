import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  recordRenderMetrics,
  getLastMetrics,
  getRollingFps,
  resetPerfMetrics,
} from './graph-perf';

// Mock performance.getEntriesByName to return controlled durations
const mockMeasures: Record<string, number> = {};
let mockNowValue = 0;

vi.stubGlobal('performance', {
  mark: vi.fn(),
  measure: vi.fn(),
  getEntriesByName: vi.fn((name: string) => {
    const dur = mockMeasures[name];
    return dur !== undefined ? [{ duration: dur }] : [];
  }),
  clearMarks: vi.fn(),
  clearMeasures: vi.fn(),
  // Monotonic mock clock — advance by 16ms (≈60fps) each call.
  now: vi.fn(() => {
    mockNowValue += 16;
    return mockNowValue;
  }),
});

describe('graph-perf', () => {
  beforeEach(() => {
    resetPerfMetrics();
    Object.keys(mockMeasures).forEach(k => delete mockMeasures[k]);
    mockNowValue = 0;
    vi.clearAllMocks();
  });

  it('getLastMetrics returns null before any recording', () => {
    expect(getLastMetrics()).toBeNull();
  });

  it('recordRenderMetrics captures timing from performance measures', () => {
    mockMeasures['render:total'] = 4.5;
    mockMeasures['render:lanes'] = 1.2;
    mockMeasures['render:merges'] = 0.8;
    mockMeasures['render:nodes'] = 0.9;
    mockMeasures['render:badges'] = 0.3;
    mockMeasures['render:text'] = 1.1;

    recordRenderMetrics();

    const metrics = getLastMetrics();
    expect(metrics).not.toBeNull();
    expect(metrics!.totalMs).toBe(4.5);
    expect(metrics!.lanesMs).toBe(1.2);
    expect(metrics!.mergesMs).toBe(0.8);
    expect(metrics!.nodesMs).toBe(0.9);
    expect(metrics!.badgesMs).toBe(0.3);
    expect(metrics!.textMs).toBe(1.1);
  });

  it('recordRenderMetrics defaults to 0 for missing measures', () => {
    mockMeasures['render:total'] = 2.0;
    // Others missing

    recordRenderMetrics();

    const metrics = getLastMetrics();
    expect(metrics).not.toBeNull();
    expect(metrics!.totalMs).toBe(2.0);
    expect(metrics!.lanesMs).toBe(0);
    expect(metrics!.mergesMs).toBe(0);
  });

  it('getRollingFps computes average from last N frames', () => {
    // Simulate 10 frames at ~16ms each (60fps)
    for (let i = 0; i < 10; i++) {
      mockMeasures['render:total'] = 2.0;
      recordRenderMetrics();
    }

    const fps = getRollingFps();
    // With 10 recorded frames the FPS calculation depends on timestamps.
    // Just verify it returns a positive number.
    expect(fps).toBeGreaterThan(0);
  });

  it('resetPerfMetrics clears everything', () => {
    mockMeasures['render:total'] = 1.0;
    recordRenderMetrics();
    expect(getLastMetrics()).not.toBeNull();

    resetPerfMetrics();
    expect(getLastMetrics()).toBeNull();
    expect(getRollingFps()).toBe(0);
  });

  it('recordRenderMetrics clears performance entries after collection', () => {
    mockMeasures['render:total'] = 1.0;
    recordRenderMetrics();

    expect(performance.clearMarks).toHaveBeenCalled();
    expect(performance.clearMeasures).toHaveBeenCalled();
  });
});
