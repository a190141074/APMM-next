// 预设详情随目录同步失效重读，持久内容由后端维护。
import { useQuery } from "@tanstack/react-query";
import { managementService } from "../core/services/management";
export function usePresetDetail(id: string) {
  return useQuery({
    queryKey: ["preset-detail", id],
    queryFn: () => managementService.detail(id),
    enabled: !!id,
    retry: false,
  });
}
export function usePresetService() {
  return managementService;
}
