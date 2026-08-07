/**
 * 应用路由（hash 模式）。
 *
 * 桌面 WebView 的静态资源协议对 history 模式刷新子路由支持不稳定，
 * 因此使用 hash 模式，刷新 / 深层链接始终安全。视图按需懒加载，便于分包。
 */

import { createRouter, createWebHashHistory } from "vue-router";
import type { RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/tasks" },
  {
    path: "/tasks",
    name: "tasks",
    component: () => import("@/presentation/views/TasksView.vue"),
    meta: { title: "任务管理" },
  },
  {
    path: "/about",
    name: "about",
    component: () => import("@/presentation/views/AboutView.vue"),
    meta: { title: "关于" },
  },
  {
    path: "/:pathMatch(.*)*",
    redirect: "/tasks",
  },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

router.afterEach((to) => {
  const title = to.meta.title as string | undefined;
  document.title = title ? `${title} · Tauri Enterprise` : "Tauri Enterprise";
});

export default router;
