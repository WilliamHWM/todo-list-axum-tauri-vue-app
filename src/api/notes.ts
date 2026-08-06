import { request } from "./http";
import type { CreateNoteInput, Note, UpdateNoteInput } from "@/types/note";

export const fetchNotesByTask = (taskId: string) =>
  request<Note[]>({ url: `/tasks/${encodeURIComponent(taskId)}/notes` });

export const createNote = (input: CreateNoteInput) =>
  request<Note>({ url: "/notes", method: "POST", data: input });

export const updateNote = (id: string, input: UpdateNoteInput) =>
  request<Note>({
    url: `/notes/${encodeURIComponent(id)}`,
    method: "PUT",
    data: input,
  });

export const deleteNote = (id: string) =>
  request<void>({ url: `/notes/${encodeURIComponent(id)}`, method: "DELETE" });
