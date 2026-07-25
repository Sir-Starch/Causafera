/**
 * A minimal selector store built on `useSyncExternalStore`.
 *
 * The observer needs three things a plain `useState` hook does not give: components that
 * subscribe to a slice rather than the whole session, feeds that only run while a consumer
 * is mounted (`docs/observer/backpressure.md` — a closed panel receives no updates), and a
 * state container that can be driven from outside React by the session controller.
 *
 * No external state library is used; the surface below is the whole contract.
 */

import { useCallback, useEffect, useSyncExternalStore } from "react";

export type Listener = () => void;

export class Store<State> {
  private state: State;
  private readonly listeners = new Set<Listener>();

  constructor(initial: State) {
    this.state = initial;
  }

  get snapshot(): State {
    return this.state;
  }

  subscribe = (listener: Listener): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  /** Apply a partial update. Reference equality of untouched slices is preserved. */
  patch(update: Partial<State> | ((current: State) => Partial<State>)): void {
    const partial = typeof update === "function" ? update(this.state) : update;
    let changed = false;
    for (const key of Object.keys(partial) as (keyof State)[]) {
      if (!Object.is(this.state[key], partial[key])) {
        changed = true;
        break;
      }
    }
    if (!changed) return;
    this.state = { ...this.state, ...partial };
    for (const listener of this.listeners) listener();
  }
}

/**
 * Read a derived slice of the store.
 *
 * The subscription is to the whole state — `patch` preserves the identity of untouched
 * slices, so the snapshot the store hands back is stable between notifications — and the
 * selector runs during render. Selectors must therefore stay pure and cheap; anything that
 * reduces a payload (`buildAtlas`, `buildLadders`) belongs behind a `useMemo` in the caller,
 * keyed by the protocol object it reduces.
 */
export function useStoreSelector<State, Slice>(
  store: Store<State>,
  select: (state: State) => Slice,
): Slice {
  const read = useCallback(() => store.snapshot, [store]);
  return select(useSyncExternalStore(store.subscribe, read, read));
}

/**
 * A reference-counted demand registry.
 *
 * A feed is only polled while at least one mounted component asks for it. This is the
 * frontend half of scoped subscriptions: closing a panel stops its traffic.
 */
export class DemandRegistry<Feed extends string> {
  private readonly counts = new Map<Feed, number>();
  private readonly listeners = new Set<Listener>();

  subscribe = (listener: Listener): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  demanded(feed: Feed): boolean {
    return (this.counts.get(feed) ?? 0) > 0;
  }

  acquire(feed: Feed): () => void {
    const next = (this.counts.get(feed) ?? 0) + 1;
    this.counts.set(feed, next);
    if (next === 1) this.notify();
    return () => {
      const remaining = (this.counts.get(feed) ?? 1) - 1;
      this.counts.set(feed, Math.max(0, remaining));
      if (remaining <= 0) this.notify();
    };
  }

  private notify(): void {
    for (const listener of this.listeners) listener();
  }
}

/** Declare that this component needs a feed for as long as it is mounted. */
export function useFeedDemand<Feed extends string>(
  registry: DemandRegistry<Feed>,
  feed: Feed,
  enabled = true,
): void {
  const acquire = useCallback(() => registry.acquire(feed), [registry, feed]);
  useEffect(() => {
    if (!enabled) return undefined;
    return acquire();
  }, [acquire, enabled]);
}
