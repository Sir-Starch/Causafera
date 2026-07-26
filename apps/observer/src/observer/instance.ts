/**
 * The single observer session for this window, plus the hooks that read it.
 *
 * The controller is a module singleton rather than component state: the connection outlives
 * any particular view, and React's development double-mount must not open two sessions.
 */

import { useCallback } from "react";

import { copyFor, type Copy } from "../i18n";
import { ObserverSessionController, resolveChannel, type Feed, type SessionState } from "./session";
import { useFeedDemand, useStoreSelector } from "./store";
import type { ByteChannel } from "./transport";

let devChannel: ByteChannel | undefined;

/** Installed by `main.tsx` in development builds only; see `src/dev/replayChannel.ts`. */
export function installDevChannel(channel: ByteChannel | undefined): void {
  devChannel = channel;
}

export const session = new ObserverSessionController(() => resolveChannel(devChannel));

export function useSession<Slice>(select: (state: SessionState) => Slice): Slice {
  return useStoreSelector(session.store, select);
}

export function useFeed(feed: Feed, enabled = true): void {
  useFeedDemand(session.demand, feed, enabled);
}

export function useCopy(): Copy {
  return copyFor(useSession((state) => state.locale));
}

export function useLocale() {
  return useSession((state) => state.locale);
}

/** Stable action bundle; components never reach into the controller's internals. */
export function useActions() {
  const toggleRun = useCallback(() => session.toggleRun(), []);
  const step = useCallback(() => void session.step(), []);
  const reset = useCallback(() => void session.reset(), []);
  const analyze = useCallback(() => void session.analyze(), []);
  const reconnect = useCallback(() => void session.connect(), []);
  return { toggleRun, step, reset, analyze, reconnect };
}
