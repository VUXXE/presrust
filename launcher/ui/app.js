// PrestaShop Portable - Tauri Frontend Logic

const invoke = (cmd, args) => {
  if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
    return window.__TAURI__.core.invoke(cmd, args);
  }
  console.warn("Tauri API not detected, running mock for command:", cmd, args);
  return Promise.resolve({});
};

// State
let appState = {
  isRunning: false,
  isSetupMode: true,
  isBusy: false,
  hostUrl: "http://127.0.0.1:8080",
  adminFolder: null,
  dbHost: "127.0.0.1",
  dbPort: 3306,
  dbUser: "root",
  dbName: "prestashop",
  webPort: 8080,
  phpPort: 9000
};

let logHistory = [];
let activeLogFilter = "ALL";
let isLogViewActive = false;

// DOM Elements
const appTitle = document.getElementById("appTitle");

const dashboardView = document.getElementById("dashboardView");
const logMonitorView = document.getElementById("logMonitorView");
const settingsModal = document.getElementById("settingsModal");

const statusDot = document.getElementById("statusDot");
const statusText = document.getElementById("statusText");
const hostValue = document.getElementById("hostValue");
const adminRow = document.getElementById("adminRow");
const adminFolderText = document.getElementById("adminFolderText");

const dbCard = document.getElementById("dbCard");
const btnCopyDb = document.getElementById("btnCopyDb");
const copyDbText = document.getElementById("copyDbText");
const dbHost = document.getElementById("dbHost");
const dbUser = document.getElementById("dbUser");
const dbName = document.getElementById("dbName");
const dbPort = document.getElementById("dbPort");

const btnMainAction = document.getElementById("btnMainAction");
const mainActionIcon = document.getElementById("mainActionIcon");
const mainActionText = document.getElementById("mainActionText");

const btnOpenShop = document.getElementById("btnOpenShop");
const shopBtnIcon = document.getElementById("shopBtnIcon");
const shopBtnText = document.getElementById("shopBtnText");

const btnOpenAdmin = document.getElementById("btnOpenAdmin");
const adminBtnIcon = document.getElementById("adminBtnIcon");
const adminBtnText = document.getElementById("adminBtnText");

const setupFootnote = document.getElementById("setupFootnote");

// Log View Elements
const logHostValue = document.getElementById("logHostValue");
const btnLogStop = document.getElementById("btnLogStop");
const btnLogShop = document.getElementById("btnLogShop");
const btnLogAdmin = document.getElementById("btnLogAdmin");
const logOutput = document.getElementById("logOutput");
const terminalConsole = document.getElementById("terminalConsole");
const chkAutoScroll = document.getElementById("chkAutoScroll");
const btnClearLogs = document.getElementById("btnClearLogs");
const btnCopyLogs = document.getElementById("btnCopyLogs");
const copyLogsText = document.getElementById("copyLogsText");

// Footer Elements
const btnToggleSettings = document.getElementById("btnToggleSettings");
const btnOpenLogsFolder = document.getElementById("btnOpenLogsFolder");
const versionTag = document.getElementById("versionTag");
const btnToggleLog = document.getElementById("btnToggleLog");
const toggleLogArrow = document.getElementById("toggleLogArrow");
const toggleLogText = document.getElementById("toggleLogText");

// Settings Form
const inputWebPort = document.getElementById("inputWebPort");
const inputPhpPort = document.getElementById("inputPhpPort");
const inputDbPort = document.getElementById("inputDbPort");
const btnCancelSettings = document.getElementById("btnCancelSettings");
const btnSaveSettings = document.getElementById("btnSaveSettings");

// Native window controls (minimize / close) are provided by the OS
// via Tauri `decorations: true`. No JS wiring needed. Service cleanup
// on native close is handled in Rust via `CloseRequested`.

// 2. UI Render based on state
function renderUI() {
  const { isRunning, isSetupMode, isBusy, hostUrl, adminFolder, dbPort: port, webPort, phpPort } = appState;

  // Header Title
  appTitle.textContent = isLogViewActive ? "PrestaShop Portable — Log Monitor" : "PrestaShop Portable";

  // Views Toggle
  dashboardView.style.display = isLogViewActive ? "none" : "flex";
  logMonitorView.style.display = isLogViewActive ? "flex" : "none";

  // Status & Host in Dashboard
  if (isRunning) {
    statusDot.className = "status-dot green";
    statusText.className = "status-text green";
    statusText.textContent = "Running";
    hostValue.textContent = hostUrl;
  } else {
    statusDot.className = "status-dot";
    statusText.className = "status-text";
    statusText.textContent = isBusy ? "Processing..." : "Stopped";
    hostValue.textContent = `127.0.0.1:${webPort}`;
  }

  // Admin Row (State 5.3)
  if (isRunning && !isSetupMode && adminFolder) {
    adminRow.style.display = "flex";
    adminFolderText.textContent = `/${adminFolder}/`;
  } else {
    adminRow.style.display = "none";
  }

  // DB Card & Footnote (State 5.2)
  if (isRunning && isSetupMode) {
    dbCard.style.display = "flex";
    setupFootnote.style.display = "flex";
    dbPort.textContent = String(port);
  } else {
    dbCard.style.display = "none";
    setupFootnote.style.display = "none";
  }

  // Big Action Button
  if (isRunning) {
    btnMainAction.className = "btn-primary-action running";
    mainActionIcon.src = "icons/stop-white.svg";
    mainActionText.textContent = isBusy ? "Stopping..." : "Stop Services";
  } else {
    btnMainAction.className = "btn-primary-action";
    mainActionIcon.src = "icons/play-white.svg";
    mainActionText.textContent = isBusy ? "Starting..." : "Start Services";
  }
  btnMainAction.disabled = isBusy;

  // Secondary Buttons
  if (!isRunning) {
    // State 5.1: Idle
    btnOpenShop.className = "btn-secondary disabled";
    btnOpenShop.disabled = true;
    shopBtnIcon.src = "icons/store-gray.svg";
    shopBtnText.textContent = "Open Shop (off)";

    btnOpenAdmin.className = "btn-secondary disabled";
    btnOpenAdmin.disabled = true;
    adminBtnIcon.src = "icons/user-gray.svg";
    adminBtnText.textContent = "Admin Login (off)";
  } else if (isSetupMode) {
    // State 5.2: Setup active
    btnOpenShop.className = "btn-secondary setup-mode";
    btnOpenShop.disabled = false;
    shopBtnIcon.src = "icons/globe-white.svg";
    shopBtnText.textContent = "Start Shop Setup";

    btnOpenAdmin.className = "btn-secondary disabled";
    btnOpenAdmin.disabled = true;
    adminBtnIcon.src = "icons/lock-gray.svg";
    adminBtnText.textContent = "Admin Login (Locked)";
  } else {
    // State 5.3: Ready
    btnOpenShop.className = "btn-secondary active-outline";
    btnOpenShop.disabled = false;
    shopBtnIcon.src = "icons/store-blue.svg";
    shopBtnText.textContent = "Open Shop";

    btnOpenAdmin.className = "btn-secondary active-outline";
    btnOpenAdmin.disabled = !adminFolder;
    adminBtnIcon.src = "icons/user-blue.svg";
    adminBtnText.textContent = "Admin Login";
  }

  // Footer Controls
  btnOpenLogsFolder.style.display = isLogViewActive ? "flex" : "none";
  if (!isRunning && !isLogViewActive) {
    versionTag.style.display = "block";
    btnToggleLog.style.display = "none";
  } else {
    versionTag.style.display = "none";
    btnToggleLog.style.display = "flex";
    toggleLogArrow.textContent = isLogViewActive ? "▲" : "▼";
    toggleLogText.textContent = isLogViewActive ? "Hide" : "Show Logs";
  }

  // Compact Log Summary
  logHostValue.textContent = hostUrl;
  btnLogAdmin.disabled = !isRunning || isSetupMode || !adminFolder;
}

// 3. Main Action Trigger
btnMainAction.addEventListener("click", async () => {
  if (appState.isBusy) return;
  appState.isBusy = true;
  renderUI();

  try {
    if (appState.isRunning) {
      await invoke("stop_services");
      appState.isRunning = false;
    } else {
      await invoke("start_services");
      appState.isRunning = true;
    }
  } catch (err) {
    alert("Error: " + err);
  } finally {
    appState.isBusy = false;
    await syncState();
  }
});

btnLogStop.addEventListener("click", async () => {
  btnMainAction.click();
});

// 4. Open Browser Handlers
btnOpenShop.addEventListener("click", () => invoke("open_shop"));
btnLogShop.addEventListener("click", () => invoke("open_shop"));
btnOpenAdmin.addEventListener("click", () => invoke("open_admin"));
btnLogAdmin.addEventListener("click", () => invoke("open_admin"));
btnOpenLogsFolder.addEventListener("click", () => invoke("open_logs_folder"));

// 5. Copy Helpers
btnCopyDb.addEventListener("click", () => {
  const text = `Host: ${appState.dbHost}\nPort: ${appState.dbPort}\nUser: ${appState.dbUser}\nPassword: \nDatabase: ${appState.dbName}`;
  navigator.clipboard.writeText(text);
  copyDbText.textContent = "Copied!";
  setTimeout(() => { copyDbText.textContent = "Copy Info"; }, 2000);
});

btnCopyLogs.addEventListener("click", () => {
  navigator.clipboard.writeText(logOutput.textContent);
  copyLogsText.textContent = "Copied!";
  setTimeout(() => { copyLogsText.textContent = "Copy Log"; }, 2000);
});

btnClearLogs.addEventListener("click", async () => {
  logHistory = [];
  logOutput.textContent = "";
  await invoke("clear_logs");
});

// 6. Log Filter & Rendering
document.querySelectorAll('input[name="logFilter"]').forEach(radio => {
  radio.addEventListener("change", (e) => {
    activeLogFilter = e.target.value;
    renderLogs();
  });
});

function renderLogs() {
  const filtered = logHistory.filter(line => {
    if (activeLogFilter === "ALL") return true;
    if (activeLogFilter === "NGINX") return line.includes("[Nginx]");
    if (activeLogFilter === "PHP") return line.includes("[PHP");
    if (activeLogFilter === "DB") return line.includes("[MariaDB]");
    return true;
  });

  logOutput.textContent = filtered.length > 0 ? filtered.join("\n") : "Waiting for activity logs...";
  if (chkAutoScroll.checked) {
    terminalConsole.scrollTop = terminalConsole.scrollHeight;
  }
}

// 7. Toggle Log View
btnToggleLog.addEventListener("click", () => {
  isLogViewActive = !isLogViewActive;
  renderUI();
  if (isLogViewActive) {
    renderLogs();
  }
});

// 8. Settings Modal
btnToggleSettings.addEventListener("click", () => {
  inputWebPort.value = appState.webPort;
  inputPhpPort.value = appState.phpPort;
  inputDbPort.value = appState.dbPort;
  settingsModal.style.display = "flex";
});

btnCancelSettings.addEventListener("click", () => {
  settingsModal.style.display = "none";
});

btnSaveSettings.addEventListener("click", async () => {
  const webPort = parseInt(inputWebPort.value, 10);
  const phpPort = parseInt(inputPhpPort.value, 10);
  const dbPort = parseInt(inputDbPort.value, 10);

  try {
    await invoke("save_settings", {
      webPort: String(webPort),
      phpPort: String(phpPort),
      dbPort: String(dbPort)
    });
    settingsModal.style.display = "none";
    await syncState();
  } catch (err) {
    alert("Failed to save ports: " + err);
  }
});

// 9. Sync State from Backend
async function syncState() {
  try {
    const res = await invoke("get_app_state");
    if (res && typeof res === "object") {
      Object.assign(appState, res);
      renderUI();
    }
  } catch (e) {
    console.error("Failed to sync app state:", e);
  }
}

// 10. Poll Logs / Events
async function fetchLogs() {
  try {
    const logs = await invoke("get_logs");
    if (Array.isArray(logs) && logs.length !== logHistory.length) {
      logHistory = logs;
      if (isLogViewActive) {
        renderLogs();
      }
    }
  } catch (e) {}
}

// Setup Event Listener if in Tauri
if (window.__TAURI__ && window.__TAURI__.event) {
  window.__TAURI__.event.listen("log-entry", (event) => {
    if (event.payload) {
      logHistory.push(event.payload);
      if (isLogViewActive) {
        renderLogs();
      }
    }
  });
}

// Init
syncState();
setInterval(syncState, 2000);
setInterval(fetchLogs, 1000);
