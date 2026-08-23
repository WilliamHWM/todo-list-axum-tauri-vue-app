import { request } from "./http";
import type {
	CategoryRepository,
	CreateCategoryInput,
	UpdateCategoryInput,
} from "@/domain/repository";
import type { Category } from "@/domain/generated";

/** 分类仓储的 HTTP 实现（南向网关）。 */
export class HttpCategoryRepository implements CategoryRepository {
	async list(): Promise<Category[]> {
		return request<Category[]>({ url: "/categories" });
	}

	async get(id: string): Promise<Category | null> {
		try {
			return await request<Category>({ url: `/categories/${encodeURIComponent(id)}` });
		} catch (err) {
			if ((err as { code?: number }).code === 404) return null;
			throw err;
		}
	}

	async create(input: CreateCategoryInput): Promise<Category> {
		return request<Category>({ url: "/categories", method: "POST", data: input });
	}

	async update(id: string, input: UpdateCategoryInput): Promise<Category> {
		return request<Category>({
			url: `/categories/${encodeURIComponent(id)}`,
			method: "PUT",
			data: input,
		});
	}

	async remove(id: string): Promise<void> {
		await request<void>({
			url: `/categories/${encodeURIComponent(id)}`,
			method: "DELETE",
		});
	}
}
