import { t } from "../shared/text";

/** 42 秒 / 8 分 / 2 小时 5 分 — list-sized durations that change at most once a minute after the first. */
export function shortDuration(milliseconds: number): string {
  if (!Number.isFinite(milliseconds) || milliseconds < 0) return "";
  const seconds = Math.floor(milliseconds / 1000);
  if (seconds < 60) return t("secondaryUi.sec", { value1: seconds });
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return t("secondaryUi.min", { value1: minutes });
  return minutes % 60 ? t("secondaryUi.hoursMinutes", { hours: Math.floor(minutes / 60), minutes: minutes % 60 }) : t("secondaryUi.hours", { hours: Math.floor(minutes / 60) });
}

