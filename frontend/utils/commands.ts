import { invoke } from "@tauri-apps/api/core";
import { Settings } from "../types";

export async function getSettings() {
  const settings = await invoke<Settings>("get_settings");

  return settings;
}

export async function updateSettings(newSettings: Settings) {
  await invoke("update_settings", { newSettings });
}
