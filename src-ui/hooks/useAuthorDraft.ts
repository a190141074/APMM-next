// 编辑会话独立于页面挂载；保存串行执行，迟到响应只推进修订号，不覆盖新输入。
import { useEffect, useSyncExternalStore } from "react";
import { authorService } from "../core/services/author";
import type { AuthorDraft } from "../core/ipc/generated/AuthorDraft";
type Snapshot = {
  draft: AuthorDraft;
  saving: boolean;
  error: string;
  dirty: boolean;
};
class DraftSession {
  value: Snapshot;
  listeners = new Set<() => void>();
  generation = 0;
  timer: ReturnType<typeof setTimeout> | undefined;
  pending: Promise<void> | null = null;
  constructor(draft: AuthorDraft) {
    this.value = { draft, saving: false, error: "", dirty: false };
  }
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  get = () => this.value;
  emit(patch: Partial<Snapshot>) {
    this.value = { ...this.value, ...patch };
    this.listeners.forEach((fn) => fn());
  }
  edit = (update: (draft: AuthorDraft) => AuthorDraft) => {
    this.generation++;
    this.emit({ draft: update(this.value.draft), dirty: true, error: "" });
    clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      void this.flush().catch(() => {});
    }, 600);
  };
  flush = (): Promise<void> => {
    clearTimeout(this.timer);
    if (this.pending) return this.pending;
    if (!this.value.dirty) return Promise.resolve();
    this.pending = (async () => {
      this.emit({ saving: true, error: "" });
      try {
        while (this.value.dirty) {
          const generation = this.generation;
          const saved = await authorService.save(this.value.draft);
          this.emit({
            draft:
              generation === this.generation
                ? saved
                : {
                    ...this.value.draft,
                    revision: saved.revision,
                    updatedAt: saved.updatedAt,
                  },
            dirty: generation !== this.generation,
          });
        }
      } catch (error) {
        this.emit({ error: String(error) });
        throw error;
      } finally {
        this.pending = null;
        this.emit({ saving: false });
      }
    })();
    return this.pending;
  };
  accept = (draft: AuthorDraft) => {
    this.emit({ draft, dirty: false, error: "" });
  };
}
const sessions = new Map<string, DraftSession>();
export function useAuthorDraft(initial: AuthorDraft) {
  if (!sessions.has(initial.id))
    sessions.set(initial.id, new DraftSession(initial));
  const session = sessions.get(initial.id)!;
  const state = useSyncExternalStore(session.subscribe, session.get);
  useEffect(
    () => () => {
      void session.flush().catch(() => {});
    },
    [session],
  );
  return {
    ...state,
    edit: session.edit,
    flush: session.flush,
    accept: session.accept,
    current: () => session.get().draft,
  };
}
