import { useCallback, useEffect, useRef, useState } from "react";
import type {
  ExplanationReport,
  RuntimeSummary,
  SpatialChunkSummary,
  WorldChunkSnapshot,
} from "@causafera/observer-protocol";

import { getInitialLocale, saveLocalePreference, type ObserverLocale } from "./locales";
import { ObserverClient, hasTauriTransport, type RuntimeUpdate } from "./observerClient";

export type { ObserverLocale } from "./locales";
export type ConnectionState = "connecting" | "connected" | "unavailable" | "error";
export type PlaybackRate = 1 | 4 | 16;

export interface TimelineSample {
  ticks: bigint;
  mana: bigint;
  traces: bigint;
  physicalEvents: bigint;
}

export interface ObserverSessionModel {
  connection: ConnectionState;
  summary?: RuntimeSummary;
  world?: WorldChunkSnapshot;
  history: TimelineSample[];
  explanation?: ExplanationReport;
  error?: string;
  isPlaying: boolean;
  isAnalyzing: boolean;
  locale: ObserverLocale;
  playbackRate: PlaybackRate;
  step(): Promise<void>;
  togglePlayback(): void;
  reset(seed: number): Promise<void>;
  analyze(): Promise<void>;
  setLocale(locale: ObserverLocale): void;
  setPlaybackRate(rate: PlaybackRate): void;
  refreshWorld(): Promise<void>;
}

const TIMELINE_CAPACITY = 96;

export function useObserverSession(worldVisible: boolean): ObserverSessionModel {
  const client = useRef(new ObserverClient());
  const [connection, setConnection] = useState<ConnectionState>("connecting");
  const [summary, setSummary] = useState<RuntimeSummary>();
  const [world, setWorld] = useState<WorldChunkSnapshot>();
  const [history, setHistory] = useState<TimelineSample[]>([]);
  const [explanation, setExplanation] = useState<ExplanationReport>();
  const [error, setError] = useState<string>();
  const [isPlaying, setIsPlaying] = useState(false);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [locale, setLocaleState] = useState<ObserverLocale>(() => getInitialLocale());
  const [playbackRate, setPlaybackRate] = useState<PlaybackRate>(4);

  const applyUpdate = useCallback((update: RuntimeUpdate) => {
    setSummary(update.summary);
    setHistory((current) => {
      const sample: TimelineSample = {
        ticks: update.summary.simulationTicks,
        mana: update.summary.manaTotal,
        traces: update.summary.causalTraceCount,
        physicalEvents: update.summary.physicalEvents,
      };
      const withoutDuplicate = current.filter((item) => item.ticks !== sample.ticks);
      return [...withoutDuplicate, sample].slice(-TIMELINE_CAPACITY);
    });
  }, []);

  const refreshWorld = useCallback(async () => {
    if (!hasTauriTransport()) return;
    try {
      setWorld(await client.current.world());
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    if (!hasTauriTransport()) {
      setConnection("unavailable");
      return undefined;
    }
    const initialize = async () => {
      try {
        setConnection("connecting");
        await client.current.connect(locale);
        const update = await client.current.openRuntimeStream();
        if (cancelled) return;
        applyUpdate(update);
        setWorld(await client.current.world());
        if (!cancelled) setConnection("connected");
      } catch (cause) {
        if (!cancelled) {
          setConnection("error");
          setError(errorMessage(cause));
        }
      }
    };
    void initialize();
    return () => {
      cancelled = true;
    };
  }, [applyUpdate, locale]);

  useEffect(() => {
    if (connection !== "connected") return;
    void client.current.connect(locale).catch((cause: unknown) => setError(errorMessage(cause)));
  }, [connection, locale]);

  useEffect(() => {
    if (connection === "connected" && worldVisible) void refreshWorld();
  }, [connection, refreshWorld, worldVisible]);

  useEffect(() => {
    if (!isPlaying || connection !== "connected") return undefined;
    let cancelled = false;
    let timer: number | undefined;
    const run = async () => {
      try {
        const update = await client.current.advance(playbackRate);
        if (cancelled) return;
        applyUpdate(update);
        if (worldVisible) setWorld(await client.current.world());
        if (!cancelled) timer = window.setTimeout(run, 420);
      } catch (cause) {
        if (!cancelled) {
          setIsPlaying(false);
          setConnection("error");
          setError(errorMessage(cause));
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [applyUpdate, connection, isPlaying, playbackRate, worldVisible]);

  const step = useCallback(async () => {
    if (connection !== "connected") return;
    setIsPlaying(false);
    setError(undefined);
    try {
      applyUpdate(await client.current.advance(1));
      if (worldVisible) setWorld(await client.current.world());
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }, [applyUpdate, connection, worldVisible]);

  const reset = useCallback(
    async (seed: number) => {
      if (connection !== "connected") return;
      setIsPlaying(false);
      setError(undefined);
      setExplanation(undefined);
      try {
        const update = await client.current.reset(seed);
        setHistory([]);
        applyUpdate(update);
        if (worldVisible) setWorld(await client.current.world());
      } catch (cause) {
        setError(errorMessage(cause));
      }
    },
    [applyUpdate, connection, worldVisible],
  );

  const analyze = useCallback(async () => {
    if (connection !== "connected" || isAnalyzing) return;
    setIsPlaying(false);
    setIsAnalyzing(true);
    setError(undefined);
    try {
      setExplanation(await client.current.analyze());
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsAnalyzing(false);
    }
  }, [connection, isAnalyzing]);

  const setLocale = useCallback((nextLocale: ObserverLocale) => {
    saveLocalePreference(nextLocale);
    setLocaleState(nextLocale);
  }, []);

  return {
    connection,
    summary,
    world,
    history,
    explanation,
    error,
    isPlaying,
    isAnalyzing,
    locale,
    playbackRate,
    step,
    togglePlayback: () => setIsPlaying((current) => !current),
    reset,
    analyze,
    setLocale,
    setPlaybackRate,
    refreshWorld,
  };
}

export function chunkKey(chunk: SpatialChunkSummary): string {
  return `${chunk.chartId}:${chunk.chunkX}:${chunk.chunkY}:${chunk.chunkZ}`;
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
