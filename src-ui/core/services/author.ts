import { appService as api } from "./app";
import { IPC } from "../ipc/commands";
import type { AuthorState } from "../ipc/generated/AuthorState";
import type { AuthorDraft } from "../ipc/generated/AuthorDraft";
import type { AuthorPreview } from "../ipc/generated/AuthorPreview";
export const authorService = {
  state: () => api.command<AuthorState>(IPC.author_state),
  syncPublished: (id: string) =>
    api.command<AuthorState>(IPC.author_sync_published, { id }),
  image: (sha256: string) => api.command<string>(IPC.author_image, { sha256 }),
  service: () =>
    api.command<{ available: boolean; message: string }>(IPC.author_service),
  create: (workId: string | null = null) =>
    api.command<AuthorDraft>(IPC.author_new, { workId }),
  save: (draft: AuthorDraft) =>
    api.command<AuthorDraft>(IPC.author_save, { draft }),
  source: (id: string, path: string, password: string, preview = false) =>
    api.command<AuthorDraft>(IPC.author_add_source, {
      id,
      path,
      password,
      preview,
    }),
  cover: (id: string, revision: number, path: string) =>
    api.command<AuthorDraft>(IPC.author_set_cover, { id, revision, path }),
  library: (id: string, paths: string[]) =>
    api.command<AuthorDraft>(IPC.author_add_library, { id, paths }),
  preflight: (id: string) =>
    api.command<AuthorPreview>(IPC.author_preflight, { id }),
  export: (id: string, destination: string) =>
    api.command<void>(IPC.author_export, { id, destination }),
  upload: (id: string) => api.task("author-upload", "上传工坊投稿", { id }),
  refresh: (id: string) => api.command<AuthorState>(IPC.author_refresh, { id }),
  cancel: (id: string, workId: string) =>
    api.command<void>(IPC.author_cancel, { id, workId }),
  backup: (id: string, destination: string, password: string) =>
    api.command<void>(IPC.author_backup, { id, destination, password }),
  restore: (path: string, password: string) =>
    api.command<void>(IPC.author_restore, { path, password }),
};
