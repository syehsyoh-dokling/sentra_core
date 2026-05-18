function appShell(active, title, body) {
  const nav = [
    ["overview", "Dashboard", "dashboard.html"],
    ["audit", "Audit Intake", "audit.html"],
    ["bulk", "Bulk Upload Test", "bulk-upload.html"],
    ["onboarding", "User Onboarding", "onboarding.html"],
    ["asset", "Asset Registry", "asset-registry.html"],
    ["tokenization", "Tokenization", "tokenization.html"],
    ["marketplace", "Marketplace", "marketplace.html"],
    ["escrow", "Transaction & Escrow", "escrow.html"],
    ["smartcontract", "Smart Contract", "smart-contract.html"],
    ["indexer", "Blockchain Indexer", "indexer.html"],
    ["revenue", "Revenue Distribution", "revenue.html"],
    ["admin", "Provider Admin", "admin.html"]
  ];

  const navHtml = nav.map(([key, label, href]) => {
    return `<a class="nav-item ${key === active ? "active" : ""}" href="${href}">
      <span class="nav-icon">•</span> ${label}
    </a>`;
  }).join("");

  document.body.innerHTML = `
    <div class="shell">
      <aside class="sidebar">
        <div class="logo">
          <img class="logo-icon" src="assets/logo-blockchain.png" alt="SentraCore Logo" onerror="this.src='assets/favicon.png'" />
          SentraCore
        </div>

        <div class="nav-section">Platform</div>
        ${navHtml}

        <div class="nav-section">Network</div>
        <a class="nav-item" href="#"><span class="nav-icon">•</span> Sepolia Testnet <span class="badge" style="margin-left:auto">Live</span></a>
        <a class="nav-item" href="#"><span class="nav-icon">•</span> Polygon Amoy</a>

        <div class="nav-section">Account</div>
        <a class="nav-item" href="#"><span class="nav-icon">•</span> Settings</a>
        <a class="nav-item logout visible-logout-sidebar" href="#" onclick="sentraLogout(); return false;">
          <span class="nav-icon">•</span> Logout
        </a>
      </aside>

      <main class="main">
        <header class="topbar">
          <div class="page-title">${title}</div>
          <div class="topbar-meta">
            <span><span class="live-dot"></span> API Connected</span>
            <span>Rust Axum · PostgreSQL · Redis</span>
            <button class="visible-logout-topbar" onclick="sentraLogout(); return false;">Logout</button>
          </div>
        </header>

        <section class="content">${body}</section>
      </main>
    </div>
  `;
}

async function sentraLogout() {
  try {
    localStorage.clear();
    sessionStorage.clear();

    if ("caches" in window) {
      const names = await caches.keys();
      await Promise.all(names.map((name) => caches.delete(name)));
    }

    document.cookie.split(";").forEach((cookie) => {
      const eqPos = cookie.indexOf("=");
      const name = eqPos > -1 ? cookie.substr(0, eqPos).trim() : cookie.trim();
      if (name) {
        document.cookie = name + "=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/";
      }
    });
  } catch (err) {
    console.warn("Logout cleanup warning:", err);
  }

  window.location.replace("index.html?logout=" + Date.now());
}

function money(n) {
  return "Rp " + Number(n).toLocaleString("id-ID");
}
