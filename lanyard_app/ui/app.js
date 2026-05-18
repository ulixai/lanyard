let vaultData = [];
let projectsData = [];
let revealedPayloads = {}; 
let editingItemId = null;
let currentDynamicCategory = 'api_key';
let pinMode = 'login'; // 'login', 'create_1', 'create_2', 'reset_auth', 'reset_1', 'reset_2'
let tempPin = '';
let activeProjectId = null;
let currentIpcReqId = null;
let pendingIpcAuth = null;

function escapeHtml(unsafe) {
    if (!unsafe) return "";
    return unsafe
         .toString()
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
}

// --- ICON FACTORY (Lucide SVGs) ---
const Icons = {
    eye: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle></svg>`,
    eyeOff: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path><line x1="1" y1="1" x2="23" y2="23"></line></svg>`,
    trash: `<svg class="svg-icon" viewBox="0 0 24 24"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>`,
    edit: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path></svg>`,
    settings: `<svg class="svg-icon" viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`,
    folder: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`,
    arrowLeft: `<svg class="svg-icon" viewBox="0 0 24 24"><line x1="19" y1="12" x2="5" y2="12"></line><polyline points="12 19 5 12 12 5"></polyline></svg>`,
    key: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"></path></svg>`,
    shield: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path></svg>`,
    lock: `<svg class="svg-icon" viewBox="0 0 24 24"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg>`,
    fileCode: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><polyline points="10 13 8 15 10 17"></polyline><polyline points="14 13 16 15 14 17"></polyline></svg>`,
    link: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>`,
    quit: `<svg class="svg-icon" viewBox="0 0 24 24"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"></path><line x1="12" y1="2" x2="12" y2="12"></line></svg>`,
    plus: `<svg class="svg-icon" viewBox="0 0 24 24"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>`,
    copy: `<svg class="svg-icon" viewBox="0 0 24 24"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>`,
    ghost: `<svg class="svg-icon xl" viewBox="0 0 24 24"><path d="M9 10h.01"></path><path d="M15 10h.01"></path><path d="M12 2a8 8 0 0 0-8 8v12l3-3 2.5 2.5L12 19l2.5 2.5L17 19l3 3V10a8 8 0 0 0-8-8z"></path></svg>`,
};

function injectIcons() {
    document.querySelectorAll('.icon-slot').forEach(slot => {
        const iconName = slot.getAttribute('data-icon');
        const sizeClass = slot.getAttribute('data-size') || ''; 
        if (Icons[iconName]) {
            slot.outerHTML = Icons[iconName].replace('class="svg-icon"', `class="svg-icon ${sizeClass} ${slot.className.replace('icon-slot', '')}"`);
        }
    });
}
document.addEventListener("DOMContentLoaded", injectIcons);

// --- TOAST SYSTEM & CLIPBOARD ---
const Toast = {
    show: function(message, type = 'success') {
        let container = document.getElementById('toast-container');
        if (!container) {
            container = document.createElement('div');
            container.id = 'toast-container';
            document.body.appendChild(container);
        }
        const toast = document.createElement('div');
        toast.className = `toast toast-${type}`;
        let icon = type === 'success' ? '✔️' : type === 'error' ? '❌' : 'ℹ️';
        toast.innerHTML = `<span>${icon}</span> <span>${message}</span>`;
        container.appendChild(toast);
        setTimeout(() => {
            toast.classList.add('hiding');
            setTimeout(() => toast.remove(), 300);
        }, 3000);
    }
};

function copyToClipboard(text) {
    navigator.clipboard.writeText(text).then(() => {
        Toast.show("Copied to clipboard!", "success");
    }).catch(err => {
        Toast.show("Failed to copy text.", "error");
    });
}

// --- THEME LOGIC ---
function changeTheme(themeName) {
    document.documentElement.setAttribute('data-theme', themeName);
    localStorage.setItem('lanyard_theme', themeName);
}
const savedTheme = localStorage.getItem('lanyard_theme') || 'dark';
document.documentElement.setAttribute('data-theme', savedTheme);
document.addEventListener("DOMContentLoaded", () => {
    const selector = document.getElementById('theme-select');
    if (selector) selector.value = savedTheme;
});

// --- STATE ---
window.addEventListener('pywebviewready', async () => {
    const hasPin = await pywebview.api.has_pin();
    if (!hasPin) {
        setPinMode('create_1');
    } else {
        setPinMode('login');
    }
});

// --- NAVIGATION ---
function switchView(viewId, element) {
    // If they click the "Projects" tab while inside a project, back out cleanly
    if (viewId === 'projects' && activeProjectId) {
        closeProject();
        return; // closeProject handles the view switch
    }

    document.querySelectorAll('.nav-item').forEach(el => el.classList.remove('active'));
    if (element) element.classList.add('active');

    document.querySelectorAll('.view-section').forEach(el => el.classList.remove('active'));
    const viewEl = document.getElementById('view-' + viewId);
    if (viewEl) viewEl.classList.add('active');
}



// --- PIN LOGIC STATE MACHINE ---
function setPinMode(mode) {
    pinMode = mode;
    const title = document.getElementById('login-title');
    const desc = document.getElementById('login-desc');
    const input = document.getElementById('pin-input');
    const btn = document.getElementById('pin-btn');
    const err = document.getElementById('pin-error');
    const cancel = document.getElementById('pin-cancel');
    
    input.value = '';
    err.innerText = '';
    document.getElementById('login-overlay').classList.add('active'); 

    // Hide cancel button by default
    cancel.style.display = 'none';

    if (mode === 'login') {
        title.innerText = "Lanyard Vault";
        desc.innerText = "Enter your Master PIN to unlock the interface.";
        input.placeholder = "••••••••";
        btn.innerText = "Unlock Vault";
    } 
    else if (mode === 'create_1') {
        title.innerText = "Welcome to Lanyard";
        desc.innerText = "Create a Master PIN to secure your vault.";
        input.placeholder = "New PIN";
        btn.innerText = "Next";
    } 
    else if (mode === 'create_2') {
        title.innerText = "Confirm PIN";
        desc.innerText = "Please re-enter your new Master PIN.";
        input.placeholder = "Confirm PIN";
        btn.innerText = "Save PIN";
    } 
    else if (mode === 'reset_auth') {
        title.innerText = "Reset Master PIN";
        desc.innerText = "Enter your CURRENT Master PIN to continue.";
        input.placeholder = "Current PIN";
        btn.innerText = "Verify";
        cancel.style.display = 'block'; // Allow user to back out of settings reset
    } 
    else if (mode === 'reset_1') {
        title.innerText = "New Master PIN";
        desc.innerText = "Enter your NEW Master PIN.";
        input.placeholder = "New PIN";
        btn.innerText = "Next";
        cancel.style.display = 'block';
    } 
    else if (mode === 'reset_2') {
        title.innerText = "Confirm New PIN";
        desc.innerText = "Please re-enter your new Master PIN.";
        input.placeholder = "Confirm New PIN";
        btn.innerText = "Update PIN";
        cancel.style.display = 'block';
    }
    else if (mode === 'ipc_auth') {
        title.innerText = "Verification Required";
        desc.innerText = `Enter your Master PIN to review a request from ${pendingIpcAuth.appName}.`;
        input.placeholder = "••••••••";
        btn.innerText = "Verify";
        cancel.style.display = 'block';
    }

    
    input.focus();
}

async function submitPin() {
    const input = document.getElementById('pin-input');
    const err = document.getElementById('pin-error');
    const pin = input.value.trim();

    if (!pin) { err.innerText = "PIN cannot be empty."; return; }

    // STATE: Standard Login
    if (pinMode === 'login') {
        const valid = await pywebview.api.verify_pin(pin);
        if (valid) unlockApp();
        else { err.innerText = "Incorrect PIN."; input.value = ""; }
    } 
    
    // STATE: Authorizing a Reset from Settings
    else if (pinMode === 'reset_auth') {
        const valid = await pywebview.api.verify_pin(pin);
        if (valid) {
            setPinMode('reset_1');
        } else {
            err.innerText = "Incorrect current PIN.";
            input.value = "";
        }
    }

    // STATE: First entry of a new PIN
    else if (pinMode === 'create_1' || pinMode === 'reset_1') {
        tempPin = pin;
        setPinMode(pinMode === 'create_1' ? 'create_2' : 'reset_2');
    } 
    
    // STATE: Confirmation of a new PIN
    else if (pinMode === 'create_2' || pinMode === 'reset_2') {
        if (pin === tempPin) {
            await pywebview.api.set_pin(pin);
            Toast.show("Master PIN saved successfully.", "success");
            tempPin = '';
            unlockApp(); 
        } else {
            err.innerText = "PINs do not match. Try again.";
            // Boot them back to the first step after a short delay
            setTimeout(() => {
                setPinMode(pinMode === 'create_2' ? 'create_1' : 'reset_1');
            }, 1500);
        }
    }

    else if (pinMode === 'ipc_auth') {
        const valid = await pywebview.api.verify_pin(pin);
        if (valid) {
            document.getElementById('login-overlay').classList.remove('active');
            
            // PIN was correct! Show the actual request modal now.
            _renderIpcModal(pendingIpcAuth.reqId, pendingIpcAuth.appName, pendingIpcAuth.targetId, pendingIpcAuth.reason, pendingIpcAuth.reqCategory);
            pendingIpcAuth = null;
        } else {
            err.innerText = "Incorrect Master PIN.";
            input.value = "";
        }
    }
}

function triggerPinReset() {
    setPinMode('reset_auth');
}

function cancelPinAction() {
    // If they cancel during an IPC request, explicitly deny the app!
    if (pinMode === 'ipc_auth' && pendingIpcAuth) {
        pywebview.api.respond_to_ipc(pendingIpcAuth.reqId, false, "", pendingIpcAuth.appName, false);
        pendingIpcAuth = null;
    }
    tempPin = '';
    document.getElementById('login-overlay').classList.remove('active');
}

document.getElementById('pin-input').addEventListener('keypress', (e) => {
    if (e.key === 'Enter') submitPin();
});

async function unlockApp() {
    document.getElementById('login-overlay').classList.remove('active');
    document.getElementById('app-container').style.display = 'flex';
    loadVault();
}

// --- VAULT RENDERING ---
async function loadVault() {
    projectsData = await pywebview.api.get_projects();
    vaultData = await pywebview.api.get_vault_items();
    
    renderProjects();
    renderCurrentContext();
}

function renderCurrentContext() {
    const isProject = !!activeProjectId;

    // We pass both the category and the project ID so the lists filter correctly.
    renderCategoryList('vault-list', 'api_key', Icons.key, isProject ? "No API keys in this project." : "No base API keys.", !isProject, isProject ? activeProjectId : null);
    renderCategoryList('passwords-list', 'password', Icons.lock, isProject ? "No passwords in this project." : "No base passwords.", !isProject, isProject ? activeProjectId : null);
    renderCategoryList('env-list', 'env', Icons.fileCode, isProject ? "No environment variables in this project." : "No base environment variables.", !isProject, isProject ? activeProjectId : null);
    renderCategoryList('crypto-list', 'crypto', Icons.link, isProject ? "No key pairs in this project." : "No base key pairs.", !isProject, isProject ? activeProjectId : null);
    renderCategoryList('licenses-list', 'license', Icons.shield, isProject ? "No licenses in this project." : "No base software licenses.", !isProject, isProject ? activeProjectId : null);

    injectIcons();
}

function createEmptyState(title) {
    return `
    <div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-muted);">
        <div style="margin-bottom: 12px; opacity: 0.5;">${Icons.ghost}</div>
        <div style="font-weight: 600;">${title}</div>
        <div style="font-size: 11px; margin-top: 4px; opacity: 0.7;">Click the + button to add your first item.</div>
    </div>`;
}

function openProject(id) {
    activeProjectId = id;
    
    const p = projectsData.find(x => x.id === id);
    if (!p) return;
    
    // Inject both normal and hover states for the title
    document.getElementById('header-title').innerHTML = `
        <span class="state-normal">${escapeHtml(p.title)}</span>
        <span class="state-hover">Home</span>
    `;
    
    // Inject both normal and hover states for the description
    document.getElementById('header-desc').innerHTML = `
        <span class="state-normal">${escapeHtml(p.description) || "Project Workspace"}</span>
        <span class="state-hover text-accent">Return to Projects Dashboard</span>
    `;
    
    // Inject both normal and hover states for the icon
    document.getElementById('header-icon-container').innerHTML = `
        <div class="state-normal" style="display:flex; align-items:center; justify-content:center; width:32px; height:32px; border-radius:6px; background:var(--bg-surface-hover); border: 1px solid var(--border); color:var(--accent); flex-shrink: 0;">${Icons.folder}</div>
        <div class="state-hover" style="display:none; align-items:center; justify-content:center; width:32px; height:32px; border-radius:6px; background:var(--accent); color:#000; flex-shrink: 0;">${Icons.arrowLeft}</div>
    `;
    
    const headerEl = document.getElementById('main-sidebar-header');
    headerEl.classList.add('back-hover');
    headerEl.title = "Click to go back to Projects";

    // Re-render context lists so they display this project's items
    renderCurrentContext();
    
    // Automatically switch them into the API Keys view for this project
    switchView('vault', document.querySelector('.nav-item[onclick*="vault"]'));
}

function closeProject() {
    activeProjectId = null;
    
    document.getElementById('header-title').innerText = "Lanyard";
    document.getElementById('header-desc').innerText = "Encrypted via local OS keychain.";
    document.getElementById('header-icon-container').innerHTML = 
        `<img src="lanyard_icon.png" alt="Lanyard" class="brand-icon">`;
    
    const headerEl = document.getElementById('main-sidebar-header');
    headerEl.classList.remove('back-hover');
    headerEl.title = "";

    // Re-render context lists back to base-level
    renderCurrentContext();

    // Switch view back to projects dashboard
    switchView('projects', document.getElementById('nav-projects'));
}

function renderProjectWorkspace() {
    // Render Unified List for this project
    renderCategoryList('ws-list', null, null, "Project is empty.", false, activeProjectId);
}

function renderProjects() {
    const list = document.getElementById('projects-list');
    if (!list) return;
    list.innerHTML = "";

    if (projectsData.length === 0) {
        // Drop grid layout so Flexbox centering works
        list.style.display = 'flex';
        list.innerHTML = createEmptyState("No projects created yet.", "Click the + button to organize your keys.");
        return;
    }

    // Restore Grid Layout
    list.style.display = 'grid';
    
    projectsData.forEach(p => {
        const itemCount = vaultData.filter(i => i.project_id === p.id).length;
        const card = `
        <div class="card" style="cursor: pointer;" onclick="openProject('${p.id}')">
            <div class="card-header" style="margin-bottom: 8px;">
                <div class="card-title-group">
                    <div class="card-icon text-accent">${Icons.folder}</div>
                    <div class="card-title">${escapeHtml(p.title)}</div>
                </div>
                <div class="card-actions" onclick="event.stopPropagation()">
                    <button onclick="editProject('${p.id}')" title="Edit Project">${Icons.edit}</button>
                    <button onclick="deleteProject('${p.id}')" title="Delete Project">${Icons.trash}</button>
                </div>
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-bottom: 16px;">${escapeHtml(p.description) || "No description."}</div>
            <div style="font-size: 11px; font-weight: 700; color: var(--accent); background: rgba(16,185,129,0.1); padding: 4px 8px; border-radius: 4px; display: inline-block;">
                ${itemCount} Items
            </div>
        </div>`;
        list.insertAdjacentHTML('beforeend', card);
    });
}

function renderCategoryList(containerId, filterCategory, defaultIcon, emptyText, baseOnly = false, forceProjectId = null) {
    const list = document.getElementById(containerId);
    if (!list) return;
    list.innerHTML = "";

    let items = vaultData;
    
    // UPDATED FILTERING: Ensure we filter by category EVEN WHEN inside a project
    if (forceProjectId) {
        items = items.filter(i => i.project_id === forceProjectId && (i.category || 'api_key') === filterCategory);
    } else if (baseOnly) {
        items = items.filter(i => (i.category || 'api_key') === filterCategory && !i.project_id);
    }

    if (items.length === 0) {
        list.style.display = 'flex';
        list.innerHTML = createEmptyState(emptyText);
        return;
    }

    list.style.display = 'flex';
    
    const getCatIcon = (cat) => {
        if (cat === 'password') return Icons.lock;
        if (cat === 'env') return Icons.fileCode;
        if (cat === 'crypto') return Icons.link;
        if (cat === 'license') return Icons.shield;
        return Icons.key;
    };

    items.forEach(item => {
        const renderIcon = defaultIcon || getCatIcon(item.category);
        
        let fieldsHtml = "";
        item.fields.forEach(fKey => {
            const isRevealed = revealedPayloads[item.id] && revealedPayloads[item.id][fKey];
            const isEmail = fKey.toLowerCase().includes('email');
            
            let displayVal = "••••••••••••••••••••••••••••";
            if (isEmail && isRevealed) displayVal = revealedPayloads[item.id][fKey];
            else if (isEmail) displayVal = "(Hidden - Click Reveal)";
            else if (isRevealed) displayVal = revealedPayloads[item.id][fKey];
            
            const iconHtml = isRevealed ? Icons.eyeOff : Icons.eye;
            const copyBtnHtml = isRevealed ? `<button class="btn btn-secondary btn-icon" onclick="copyToClipboard('${revealedPayloads[item.id][fKey].replace(/'/g, "\\'")}')" title="Copy to Clipboard">${Icons.copy}</button>` : '';

            let valElement = `<div class="secret-input" title="${displayVal}">${displayVal}</div>`;
            if (isRevealed && displayVal.includes('\n')) {
                valElement = `<textarea class="secret-input" style="height: 100px; resize: vertical;" readonly>${displayVal}</textarea>`;
            }

            fieldsHtml += `
            <div class="secret-row">
                <div class="secret-label">${fKey}</div>
                ${valElement}
                <div style="display: flex; gap: 8px;">
                    ${copyBtnHtml}
                    <button class="btn btn-secondary btn-icon" onclick="toggleReveal('${item.id}')" title="Toggle Visibility">
                        ${iconHtml}
                    </button>
                </div>
            </div>`;
        });

        const card = `
        <div class="card">
            <div class="card-header">
                <div class="card-title-group">
                    <div class="card-icon text-accent">${renderIcon}</div>
                    <div class="card-title">${escapeHtml(item.title)}</div>
                </div>
                <div class="card-actions">
                    <button onclick="editItem('${item.id}', '${item.category || 'api_key'}')" title="Edit Item">${Icons.edit}</button>
                    <button onclick="deleteItem('${item.id}')" title="Delete Item">${Icons.trash}</button>
                </div>
            </div>
            <div style="border-top: 1px solid var(--border); padding-top: 16px;">
                ${fieldsHtml}
            </div>
        </div>`;
        list.insertAdjacentHTML('beforeend', card);
    });
}

async function toggleReveal(id) {
    if (revealedPayloads[id]) {
        delete revealedPayloads[id];
        loadVault();
    } else {
        const payload = await pywebview.api.reveal_payload(id);
        if (payload) {
            revealedPayloads[id] = payload;
            loadVault();
        } else {
            Toast.show("Verification Failed.", "error");
        }
    }
}

async function deleteItem(id) {
    if (confirm("Delete this item permanently?")) {
        await pywebview.api.delete_item(id);
        delete revealedPayloads[id];
        loadVault();
        Toast.show("Item deleted.", "neutral");
    }
}

// --- UNIFIED DYNAMIC MODAL (ADD & EDIT) ---
function getCategoryConfig(category) {
    const cfg = {
        project: { title: "Project", pills: ['Description'] },
        api_key: { title: "API Key", pills: ['Private/Secret Key', 'Public Key', 'Endpoint URL', 'Custom Field'] },
        password: { title: "Password", pills: ['Username', 'Password', 'Website URL', 'Notes'] },
        license: { title: "Software License", pills: ['License Key', 'Registered Email', 'Software Version'] },
        env: { title: "Environment Variables", pills: ['Custom Key/Value'] },
        crypto: { title: "Cryptographic Key Pair", pills: [] } // Handled specially
    };
    return cfg[category] || cfg.api_key;
}

function openItemSelector() { document.getElementById('selector-modal').classList.add('active'); }
function closeItemSelector() { document.getElementById('selector-modal').classList.remove('active'); }
function closeSelectorAndOpen(category) { closeItemSelector(); openDynamicModal(category); }

async function editProject(id) {
    const p = projectsData.find(x => x.id === id);
    if (!p) return;
    openDynamicModal('project');
    editingItemId = id;
    document.getElementById('dynamic-title').value = p.title;
    if (p.description) addField('Description', p.description);
}

async function deleteProject(id) {
    if (confirm("Delete this Project AND all keys inside it?")) {
        await pywebview.api.delete_project(id);
        loadVault();
        Toast.show("Project deleted.", "neutral");
    }
}

async function openDynamicModal(category) {
    editingItemId = null;
    currentDynamicCategory = category;
    const cfg = getCategoryConfig(category);
    
    document.getElementById('dynamic-modal-header').innerText = `Add ${cfg.title}`;
    document.getElementById('dynamic-title').value = "";
    document.getElementById('dynamic-fields-container').innerHTML = "";
    
    setupModalUI(category, cfg);
    document.getElementById('dynamic-modal').classList.add('active');
}

async function editItem(id, category) {
    editingItemId = id;
    currentDynamicCategory = category;
    const cfg = getCategoryConfig(category);
    
    document.getElementById('dynamic-modal-header').innerText = `Edit ${cfg.title}`;
    document.getElementById('dynamic-fields-container').innerHTML = "";
    setupModalUI(category, cfg);

    // Fetch secure payload and populate
    const meta = vaultData.find(i => i.id === id);
    if (!meta) return;
    document.getElementById('dynamic-title').value = meta.title;

    const payload = await pywebview.api.reveal_payload(id);
    if (payload) {
        Object.keys(payload).forEach(key => addField(key, payload[key]));
        document.getElementById('dynamic-modal').classList.add('active');
    } else {
        Toast.show("Authentication required to edit.", "error");
    }
}

function setupModalUI(category, cfg) {
    const pillsContainer = document.getElementById('dynamic-pills');
    const customUiContainer = document.getElementById('dynamic-category-ui');
    pillsContainer.innerHTML = "";
    customUiContainer.innerHTML = "";

    // Generate Standard Pills
    cfg.pills.forEach(pill => {
        pillsContainer.innerHTML += `<button class="pill" onclick="addField('${pill}')">+ ${pill}</button>`;
    });

    // Inject Custom UI for special categories
    if (category === 'env') {
        customUiContainer.innerHTML = `
            <button class="btn btn-secondary" onclick="triggerEnvUpload()" style="width: 100%; justify-content: center; margin-bottom: 24px;">
                <span class="icon-slot" data-icon="fileCode"></span> Upload .env File
            </button>
        `;
        injectIcons();
    } else if (category === 'crypto') {
        customUiContainer.innerHTML = `
            <label class="form-label">Algorithm</label>
            <div style="display: flex; gap: 12px; margin-bottom: 24px;">
                <select id="crypto-algo" style="flex: 1;">
                    <option value="ed25519">Ed25519 (Fast, Secure, Recommended)</option>
                    <option value="rsa-2048">RSA 2048-bit</option>
                    <option value="rsa-4096">RSA 4096-bit</option>
                </select>
                <button class="btn btn-secondary" onclick="generateKeys()">Generate</button>
            </div>
            <button class="btn btn-secondary" onclick="addManualKeypair()" style="width: 100%; justify-content: center; margin-bottom: 24px;">
                <span class="icon-slot" data-icon="edit"></span> Enter Keys Manually
            </button>
        `;
        injectIcons();
    }
}

function closeDynamicModal() {
    document.getElementById('dynamic-modal').classList.remove('active');
    editingItemId = null;
}

function addField(label, value = "") {
    const container = document.getElementById('dynamic-fields-container'); 
    if (!container) {
        console.error("Could not find dynamic-fields-container");
        return;
    }

    const isCustom = label === "Custom Field" || label === "Custom Key/Value";
    
    let labelHtml = '';
    let keyAttr = '';
    
    if (isCustom) {
        // INLINE EDITABLE KEY NAME
        labelHtml = `<input type="text" class="custom-key-input" placeholder="TYPE FIELD NAME HERE...">`;
        keyAttr = `data-custom="true"`;
    } else {
        // STANDARD FIXED LABEL
        labelHtml = `<label class="form-label text-accent" style="margin-bottom: 8px;">${label}</label>`;
        keyAttr = `data-key="${label}"`;
    }

    // STRICTLY 1-LINE INPUT
    const inputHtml = `<input type="text" class="dynamic-input" ${keyAttr} value="${value}" placeholder="Paste secret here...">`;

    // Wrapped in a distinct box so the X button aligns perfectly
    const html = `
    <div class="dynamic-field-group" style="position: relative; background: var(--bg-app); padding: 16px; border: 1px solid var(--border); border-radius: 4px;">
        ${labelHtml}
        ${inputHtml}
        <button type="button" class="btn-icon text-danger" style="position: absolute; top: 12px; right: 12px; background: transparent; border: none; cursor: pointer; width: 24px; height: 24px; padding: 0;" onclick="this.parentElement.remove()">✕</button>
    </div>`;
    
    container.insertAdjacentHTML('beforeend', html);
}

// --- SPECIAL CATEGORY LOGIC ---
async function triggerEnvUpload() {
    const res = await pywebview.api.load_env_file();
    if (res.status === 'success') {
        document.getElementById('dynamic-fields-container').innerHTML = ""; // Clear existing
        Object.keys(res.data).forEach(key => addField(key, res.data[key]));
        Toast.show("Environment file parsed.", "success");
    } else if (res.status === 'error') {
        Toast.show(res.message, "error");
    }
}

async function generateKeys() {
    const algo = document.getElementById('crypto-algo').value;
    const res = await pywebview.api.generate_keypair(algo);
    if (res.status === 'success') {
        document.getElementById('dynamic-fields-container').innerHTML = ""; // Clear existing
        addField("Public Key", res.public_key);
        addField("Private Key", res.private_key);
        Toast.show("Secure Key Pair Generated.", "success");
    } else {
        Toast.show("Generation Failed: " + res.message, "error");
    }
}

function addManualKeypair() {
    // Clear any existing fields just in case they clicked generate first
    document.getElementById('dynamic-fields-container').innerHTML = ""; 
    
    // Spawn the two empty inputs
    addField("Public Key", "");
    addField("Private Key", "");
}

// --- SAVE ROUTINE ---
async function saveDynamicItem() {
    const title = document.getElementById('dynamic-title').value.trim();
    if (!title) { Toast.show("Please enter a Title.", "error"); return; }

    const inputs = document.querySelectorAll('.dynamic-input');
    
    // Project saves are special (No Keyring, just metadata)
    if (currentDynamicCategory === 'project') {
        let desc = "";
        inputs.forEach(inp => { if (inp.getAttribute('data-key') === 'Description') desc = inp.value.trim(); });
        await pywebview.api.save_project(editingItemId, title, desc);
        closeDynamicModal();
        loadVault();
        Toast.show("Project saved.", "success");
        return;
    }

    // Standard Vault Item Save
    if (inputs.length === 0) { Toast.show("Please add at least one field.", "error"); return; }

    const fields = {};
    let hasError = false;
    inputs.forEach(inp => {
        let k = inp.getAttribute('data-key');
        if (inp.getAttribute('data-custom') === "true") {
            const customKeyInp = inp.parentElement.querySelector('.custom-key-input');
            k = customKeyInp ? customKeyInp.value.trim() : "";
            if (!k) { hasError = true; customKeyInp.style.borderBottom = "1px solid var(--danger)"; }
        }
        const v = inp.value.trim();
        if (k && v) fields[k] = v;
    });

    if (hasError) { Toast.show("Custom fields must have a name.", "error"); return; }
    if (Object.keys(fields).length === 0) { Toast.show("Fields cannot be empty.", "error"); return; }

    // Pass activeProjectId so it gets bound to the current project if we are inside one!
    await pywebview.api.save_vault_item(editingItemId, title, fields, currentDynamicCategory, activeProjectId);
    
    closeDynamicModal();
    loadVault();
    Toast.show("Item securely saved.", "success");
}


// --- ACCESS ---
window.showAccessRequest = function(reqId, appName, targetId, reason, reqCategory) {
    // 1. Store the request in memory
    pendingIpcAuth = { reqId, appName, targetId, reason, reqCategory };
    
    // 2. Force the Master PIN overlay to appear first!
    setPinMode('ipc_auth');
}

// 3. This is the old showAccessRequest code, renamed. It is called by submitPin()!
function _renderIpcModal(reqId, appName, targetId, reason, reqCategory) {
    currentIpcReqId = reqId; 
    
    document.getElementById('req-app-name').innerText = appName;
    document.getElementById('req-app-name-2').innerText = appName;

    const reasonEl = document.getElementById('req-reason');
    if (reason) {
        reasonEl.innerText = reason;
        reasonEl.title = reason;
        reasonEl.style.display = 'block';
    } else {
        reasonEl.style.display = 'none';
    }
    
    const titleEl = document.getElementById('req-key-title');
    const selectEl = document.getElementById('req-key-select');

    if (targetId) {
        // --- DIRECT REQUEST MODE ---
        let itemTitle = "Unknown Key";
        const item = vaultData.find(i => i.id === targetId);
        if (item) itemTitle = item.title;

        titleEl.innerText = itemTitle;
        titleEl.style.display = 'inline-block';
        selectEl.style.display = 'none';
        
        document.getElementById('ipc-modal').setAttribute('data-target', targetId);
    } else {
        // --- LINK REQUEST MODE (Tree Structure Dropdown) ---
        titleEl.style.display = 'none';
        selectEl.style.display = 'inline-block';
        
        let filteredVault = vaultData;
        if (reqCategory) {
            filteredVault = vaultData.filter(i => (i.category || 'api_key') === reqCategory);
        }
        
        if (filteredVault.length > 0) {
            let optionsHtml = "";
            
            const baseItems = filteredVault.filter(i => !i.project_id);
            if (baseItems.length > 0) {
                optionsHtml += `<optgroup label="Base Vault">`;
                baseItems.forEach(item => {
                    optionsHtml += `<option value="${escapeHtml(item.id)}">${escapeHtml(item.title)}</option>`;
                });
                optionsHtml += `</optgroup>`;
            }

            if (typeof projectsData !== 'undefined') {
                projectsData.forEach(proj => {
                    const projItems = filteredVault.filter(i => i.project_id === proj.id);
                    if (projItems.length > 0) {
                        optionsHtml += `<optgroup label="📁 ${escapeHtml(proj.title)}">`;
                        projItems.forEach(item => {
                            optionsHtml += `<option value="${escapeHtml(item.id)}">${escapeHtml(item.title)}</option>`;
                        });
                        optionsHtml += `</optgroup>`;
                    }
                });
            }

            if (optionsHtml === "") optionsHtml = `<option value="">-- No Keys Match Category --</option>`;
            selectEl.innerHTML = optionsHtml;
        } else {
            selectEl.innerHTML = `<option value="">-- No Keys in Vault --</option>`;
        }
        
        document.getElementById('ipc-modal').removeAttribute('data-target');
    }
    
    document.getElementById('ipc-modal').setAttribute('data-app', appName);
    document.getElementById('chk-always-allow').checked = false;
    document.getElementById('ipc-modal').classList.add('active');
}

function approveAccess() {
    const modal = document.getElementById('ipc-modal');
    const appName = modal.getAttribute('data-app');
    const alwaysAllow = document.getElementById('chk-always-allow').checked;
    
    // If data-target exists, it's a Direct Request. Otherwise, get from dropdown.
    let targetId = modal.getAttribute('data-target');
    if (!targetId) {
        targetId = document.getElementById('req-key-select').value;
    }

    if (!targetId) {
        Toast.show("You must select a key to approve access.", "error");
        return;
    }
    
    pywebview.api.respond_to_ipc(currentIpcReqId, true, targetId, appName, alwaysAllow);
    modal.classList.remove('active');
    currentIpcReqId = null;
}

function denyAccess() {
    const modal = document.getElementById('ipc-modal');
    const targetId = modal.getAttribute('data-target');
    const appName = modal.getAttribute('data-app');
    
    pywebview.api.respond_to_ipc(currentIpcReqId, false, targetId, appName, false);
    modal.classList.remove('active');
    currentIpcReqId = null;
}