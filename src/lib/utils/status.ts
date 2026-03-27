import * as m from "$lib/paraglide/messages";
import { getThemedStatusColor } from "../stores/theme";

const FALLBACK_COLORS: Record<string, string> = {
  success: "#3fb950",
  failed: "#f85149",
  running: "#58a6ff",
  pending: "#d29922",
  queued: "#d29922",
  canceled: "#666666",
  skipped: "#484f58",
  manual: "#bb80ff",
  timed_out: "#f85149",
};

export function ciStatusColor(status: string): string {
  const themed = getThemedStatusColor(status);
  return themed || FALLBACK_COLORS[status] || "#666666";
}

export function ciStatusLabel(status: string): string {
  switch (status) {
    case "success": return m.pipeline_status_success();
    case "failed": return m.pipeline_status_failed();
    case "running": return m.pipeline_status_running();
    case "pending": return m.pipeline_status_pending();
    case "queued": return m.pipeline_status_queued();
    case "canceled": return m.pipeline_status_canceled();
    case "skipped": return m.pipeline_status_skipped();
    case "manual": return m.pipeline_status_manual();
    case "timed_out": return m.pipeline_status_timed_out();
    default: return status;
  }
}
