/**
 * 笔记仓储的 HTTP 适配器：领域端口 → axios 请求。
 */

import { request } from "./http";
import type {
  CreateNoteInput,
  Note,
  NoteRepository,
  UpdateNoteInput,
} from "@/domain/repository";

/** 基于 HTTP 的 `NoteRepository` 实现。 */
export class HttpNoteRepository implements NoteRepository {
  listByTask(taskId: string): Promise<Note[]> {
    return request<Note[]>({
      url: `/tasks/${encodeURIComponent(taskId)}/notes`,
    });
  }

  create(input: CreateNoteInput): Promise<Note> {
    return request<Note>({ url: "/notes", method: "POST", data: input });
  }

  update(id: string, input: UpdateNoteInput): Promise<Note> {
    return request<Note>({
      url: `/notes/${encodeURIComponent(id)}`,
      method: "PUT",
      data: input,
    });
  }

  remove(id: string): Promise<void> {
    return request<void>({
      url: `/notes/${encodeURIComponent(id)}`,
      method: "DELETE",
    });
  }
}
