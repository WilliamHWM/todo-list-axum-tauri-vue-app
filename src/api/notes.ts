import { request } from "./http";

/** 与 Rust 的 `Note` 对应。 */
export interface Note {
  id: string;
  taskId: string | null;
  content: string;
  createdAt: string;
}

export interface UpdateNoteInput { content?: string; }

export const fetchNotesByTask = (taskId: string) =>
  request<Note[]>(`/tasks/${encodeURIComponent(taskId)}/notes`);
export const createNote = (input: { taskId?: string; content: string }) =>
  request<Note>("/notes", { method: "POST", body: JSON.stringify(input) });
export const updateNote = (id: string, input: UpdateNoteInput) =>
  request<Note>(`/notes/${encodeURIComponent(id)}`, { method: "PUT", body: JSON.stringify(input) });
export const deleteNote = (id: string) =>
  request<void>(`/notes/${encodeURIComponent(id)}`, { method: "DELETE" });
