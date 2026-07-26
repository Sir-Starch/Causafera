/**
 * Command palette.
 *
 * Areas and run controls reachable from the keyboard without learning a shortcut for each.
 * Analytical efficiency is a stated product priority; this is the cheapest large win.
 */

import { useEffect, useMemo, useRef, useState } from "react";

import { useActions, useCopy, useSession } from "../observer/instance";
import { AREA_IDS, type AreaId } from "../workspace";
import { AreaMark } from "./Sigil";
import { Kbd } from "./primitives";

interface Command {
  id: string;
  group: "areas" | "actions";
  label: string;
  hint?: string;
  area?: AreaId;
  run(): void;
}

export function CommandPalette({
  open,
  onClose,
  goTo,
}: {
  open: boolean;
  onClose(): void;
  goTo(area: AreaId): void;
}) {
  const copy = useCopy();
  const actions = useActions();
  const running = useSession((state) => state.running);
  const attached = useSession((state) => state.connection === "connected");
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<Command[]>(() => {
    const list: Command[] = AREA_IDS.map((area) => ({
      id: `area:${area}`,
      group: "areas",
      label: copy.areas[area].name,
      hint: copy.areas[area].note,
      area,
      run: () => goTo(area),
    }));
    if (attached) {
      list.push(
        {
          id: "action:run",
          group: "actions",
          label: running ? copy.transport.pause : copy.transport.run,
          hint: "Space",
          run: actions.toggleRun,
        },
        {
          id: "action:step",
          group: "actions",
          label: copy.transport.step,
          hint: "→",
          run: actions.step,
        },
        { id: "action:analyze", group: "actions", label: copy.assay.run, run: actions.analyze },
        { id: "action:reset", group: "actions", label: copy.transport.reset, run: actions.reset },
      );
    } else {
      list.push({
        id: "action:reconnect",
        group: "actions",
        label: copy.connection.reconnect,
        run: actions.reconnect,
      });
    }
    return list;
  }, [actions, attached, copy, goTo, running]);

  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (needle.length === 0) return commands;
    return commands.filter((command) =>
      `${command.label} ${command.hint ?? ""}`.toLowerCase().includes(needle),
    );
  }, [commands, query]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setCursor(0);
      const handle = window.setTimeout(() => inputRef.current?.focus(), 0);
      return () => window.clearTimeout(handle);
    }
    return undefined;
  }, [open]);

  useEffect(() => {
    setCursor((current) => Math.min(current, Math.max(0, matches.length - 1)));
  }, [matches.length]);

  if (!open) return null;

  const groups: { id: Command["group"]; label: string }[] = [
    { id: "areas", label: copy.palette.areas },
    { id: "actions", label: copy.palette.actions },
  ];

  return (
    <div
      className="palette-scrim"
      role="dialog"
      aria-modal="true"
      aria-label={copy.meridian.palette}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="palette">
        <div className="palette__field">
          <span className="eyebrow">⌘</span>
          <input
            ref={inputRef}
            value={query}
            placeholder={copy.palette.placeholder}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                onClose();
                return;
              }
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setCursor((current) => Math.min(current + 1, matches.length - 1));
                return;
              }
              if (event.key === "ArrowUp") {
                event.preventDefault();
                setCursor((current) => Math.max(current - 1, 0));
                return;
              }
              if (event.key === "Enter") {
                event.preventDefault();
                const command = matches[cursor];
                if (command !== undefined) {
                  command.run();
                  onClose();
                }
              }
            }}
          />
          <Kbd>Esc</Kbd>
        </div>
        <div className="palette__list">
          {matches.length === 0 && (
            <p className="palette__group muted" style={{ fontSize: "var(--t-small)" }}>
              {copy.palette.empty}
            </p>
          )}
          {groups.map((group) => {
            const items = matches.filter((command) => command.group === group.id);
            if (items.length === 0) return null;
            return (
              <div key={group.id}>
                <div className="palette__group eyebrow">{group.label}</div>
                {items.map((command) => {
                  const index = matches.indexOf(command);
                  return (
                    <button
                      key={command.id}
                      type="button"
                      className="palette__item"
                      data-active={index === cursor}
                      onPointerEnter={() => setCursor(index)}
                      onClick={() => {
                        command.run();
                        onClose();
                      }}
                    >
                      {command.area !== undefined && <AreaMark area={command.area} size={15} />}
                      <b>{command.label}</b>
                      {command.hint !== undefined && (
                        <span className="muted" style={{ marginLeft: "auto", fontSize: "var(--t-micro)" }}>
                          {command.hint}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
