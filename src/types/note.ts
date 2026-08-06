/** 与 Rust `Note` 对应的领域模型。 */
export interface Note {
  id: string;
  taskId: string | null;
  content: string;
  createdAt: string;
}

/** `POST /api/notes` 的请求体。 */
export interface CreateNoteInput {
  taskId?: string | null;
  content: string;
}

/** `PUT /api/notes/:id` 的请求体。 */
export interface UpdateNoteInput {
  content?: string;
}
