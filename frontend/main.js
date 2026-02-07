const { invoke } = window.__TAURI__.core;

let open_to_tray_el = null;
let settings = null;

window.addEventListener("DOMContentLoaded", () => {
  // greetInputEl = document.querySelector("#greet-input");
  // greetMsgEl = document.querySelector("#greet-msg");
  // document.querySelector("#greet-form").addEventListener("submit", (e) => {
  //   e.preventDefault();
  //   greet();
  // });

  open_to_tray_el = document.querySelector("#open-to-tray");

  open_to_tray_el.addEventListener("change", (e) => {
    settings.open_to_tray = e.target.checked;
    update_settings(settings);
  });

  handle_window_start();
});

async function handle_window_start() {
  settings = await invoke("get_settings");

  if (settings.open_to_tray) {
    await invoke("hide_window");
  }
}

async function update_settings(settings) {
  await invoke("update_settings", { settings });
}
