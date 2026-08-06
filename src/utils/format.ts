/**
 * 时间格式化工具。
 *
 * 后端统一返回 RFC 3339 UTC 时间（带 `Z`）。dayjs 会把这类字符串解析为
 * UTC 内部表示，这里通过 `utc` 插件显式调用 `.local()`，把时间转换为
 * 用户本地时区展示，保证不同时区的用户看到的是各自本地时间。
 */

import dayjs from "dayjs";
import utc from "dayjs/plugin/utc";

dayjs.extend(utc);

/** 格式化为 `YYYY-MM-DD HH:mm:ss`（本地时区）。解析失败时原样返回。 */
export function formatDateTime(value: string | Date): string {
  const date = dayjs(value);
  if (!date.isValid()) return String(value);
  return date.local().format("YYYY-MM-DD HH:mm:ss");
}
