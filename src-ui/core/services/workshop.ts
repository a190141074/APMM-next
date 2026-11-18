import { appService } from "./app";
import { IPC } from "../ipc/commands";
import type { Listing } from "../ipc/generated/Listing";
import type { InstallPlan } from "../ipc/generated/InstallPlan";
import type { InstallRequest } from "../ipc/generated/InstallRequest";
export const workshopService = {
  list: () => appService.command<Listing>(IPC.workshop_list),
  image: (id: string, name: string) =>
    appService.command<string>(IPC.workshop_image, { id, name }),
  preflight: (id: string) =>
    appService.command<InstallPlan>(IPC.workshop_preflight, { id }),
  install: (request: InstallRequest) =>
    appService.task("workshop-install", "获取工坊作品", { ...request }),
  uninstall: (id: string) =>
    appService.task("workshop-uninstall", "移除工坊作品", { id }),
};
