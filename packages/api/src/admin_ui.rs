//! UI del Panel Administrativo Integrado en Axum (HTML/CSS/JS Embebido) con Login y Expiración

pub const ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Email Validator | Panel Administrativo</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&family=Plus+Jakarta+Sans:wght@400;500;600;700;800&display=swap" rel="stylesheet">
  <style>
    :root {
      --bg: #0b0f19;
      --card-bg: rgba(18, 24, 38, 0.75);
      --card-border: rgba(255, 255, 255, 0.08);
      --card-border-hover: rgba(99, 102, 241, 0.35);
      --primary: #6366f1;
      --primary-hover: #4f46e5;
      --primary-glow: rgba(99, 102, 241, 0.25);
      --accent: #06b6d4;
      --success: #10b981;
      --warning: #f59e0b;
      --danger: #ef4444;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --font-main: 'Plus Jakarta Sans', sans-serif;
      --font-mono: 'JetBrains Mono', monospace;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      background-color: var(--bg);
      background-image: 
        radial-gradient(at 0% 0%, rgba(99, 102, 241, 0.12) 0px, transparent 50%),
        radial-gradient(at 100% 100%, rgba(6, 182, 212, 0.08) 0px, transparent 50%);
      background-attachment: fixed;
      color: var(--text);
      font-family: var(--font-main);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }

    /* LOGIN MODAL OVERLAY */
    #loginOverlay {
      position: fixed;
      inset: 0;
      background: rgba(11, 15, 25, 0.95);
      backdrop-filter: blur(16px);
      z-index: 1000;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 1.5rem;
    }

    .login-card {
      background: var(--card-bg);
      border: 1px solid var(--card-border-hover);
      border-radius: 20px;
      padding: 2.5rem;
      max-width: 420px;
      width: 100%;
      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5), 0 0 40px var(--primary-glow);
      text-align: center;
    }

    /* HEADER */
    header {
      backdrop-filter: blur(16px);
      background: rgba(11, 15, 25, 0.85);
      border-bottom: 1px solid var(--card-border);
      padding: 1rem 2rem;
      position: sticky;
      top: 0;
      z-index: 100;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .brand {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .brand-icon {
      width: 40px;
      height: 40px;
      background: linear-gradient(135deg, var(--primary), var(--accent));
      border-radius: 10px;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: 0 0 20px var(--primary-glow);
    }

    .brand-title {
      font-size: 1.25rem;
      font-weight: 800;
      letter-spacing: -0.02em;
    }

    .brand-badge {
      font-size: 0.7rem;
      background: rgba(99, 102, 241, 0.15);
      color: var(--primary);
      padding: 0.2rem 0.6rem;
      border-radius: 9999px;
      border: 1px solid rgba(99, 102, 241, 0.3);
      font-weight: 700;
    }

    /* NAV TABS */
    .nav-tabs {
      display: flex;
      gap: 0.5rem;
      background: rgba(18, 24, 38, 0.5);
      padding: 0.35rem;
      border-radius: 12px;
      border: 1px solid var(--card-border);
    }

    .tab-btn {
      background: transparent;
      border: none;
      color: var(--text-muted);
      padding: 0.6rem 1.2rem;
      font-size: 0.875rem;
      font-weight: 600;
      border-radius: 8px;
      cursor: pointer;
      transition: all 0.2s ease;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .tab-btn:hover {
      color: var(--text);
      background: rgba(255, 255, 255, 0.04);
    }

    .tab-btn.active {
      color: #fff;
      background: var(--primary);
      box-shadow: 0 0 15px var(--primary-glow);
    }

    .user-profile {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      font-size: 0.85rem;
    }

    /* MAIN CONTAINER */
    main {
      flex: 1;
      padding: 2rem;
      max-width: 1400px;
      width: 100%;
      margin: 0 auto;
    }

    .tab-content { display: none; }
    .tab-content.active { display: block; animation: fadeIn 0.3s ease; }

    @keyframes fadeIn {
      from { opacity: 0; transform: translateY(6px); }
      to { opacity: 1; transform: translateY(0); }
    }

    /* CARDS */
    .card {
      background: var(--card-bg);
      backdrop-filter: blur(12px);
      border: 1px solid var(--card-border);
      border-radius: 16px;
      padding: 1.5rem;
      margin-bottom: 1.5rem;
      transition: border-color 0.2s ease;
    }
    .card:hover { border-color: var(--card-border-hover); }

    .card-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 1.25rem;
    }
    .card-title {
      font-size: 1.1rem;
      font-weight: 700;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    /* KPI METRICS GRID */
    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 1.25rem;
      margin-bottom: 1.5rem;
    }

    .kpi-card {
      background: rgba(18, 24, 38, 0.6);
      border: 1px solid var(--card-border);
      border-radius: 14px;
      padding: 1.25rem;
      position: relative;
      overflow: hidden;
    }
    .kpi-card::after {
      content: '';
      position: absolute;
      top: 0; left: 0; right: 0; height: 2px;
      background: linear-gradient(90deg, transparent, var(--primary), transparent);
    }

    .kpi-label { font-size: 0.8rem; color: var(--text-muted); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
    .kpi-value { font-size: 2rem; font-weight: 800; margin: 0.4rem 0; font-family: var(--font-mono); }
    .kpi-sub { font-size: 0.75rem; color: var(--text-muted); display: flex; align-items: center; gap: 0.3rem; }

    /* TABLES */
    .table-responsive { overflow-x: auto; }
    table { width: 100%; border-collapse: collapse; text-align: left; }
    th {
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted);
      padding: 0.75rem 1rem;
      border-bottom: 1px solid var(--card-border);
    }
    td {
      padding: 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.04);
      font-size: 0.875rem;
    }
    tr:last-child td { border-bottom: none; }
    tr:hover td { background: rgba(255, 255, 255, 0.02); }

    /* BUTTONS & INPUTS */
    .btn {
      background: var(--primary);
      color: #fff;
      border: none;
      padding: 0.6rem 1.2rem;
      font-size: 0.875rem;
      font-weight: 600;
      border-radius: 8px;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      transition: all 0.2s ease;
      font-family: var(--font-main);
    }
    .btn:hover { background: var(--primary-hover); transform: translateY(-1px); }
    .btn-danger { background: rgba(239, 68, 68, 0.15); color: var(--danger); border: 1px solid rgba(239, 68, 68, 0.3); }
    .btn-danger:hover { background: var(--danger); color: #fff; }
    .btn-sm { padding: 0.4rem 0.8rem; font-size: 0.75rem; }

    .input-group { margin-bottom: 1.25rem; }
    label { display: block; font-size: 0.8rem; font-weight: 600; margin-bottom: 0.4rem; color: var(--text-muted); }
    input, select, textarea {
      width: 100%;
      background: rgba(11, 15, 25, 0.7);
      border: 1px solid var(--card-border);
      border-radius: 8px;
      padding: 0.75rem 1rem;
      color: var(--text);
      font-family: var(--font-main);
      font-size: 0.875rem;
      transition: border-color 0.2s ease;
    }
    input:focus, select:focus, textarea:focus {
      outline: none;
      border-color: var(--primary);
      box-shadow: 0 0 10px var(--primary-glow);
    }

    /* BADGES */
    .badge {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      padding: 0.25rem 0.65rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 600;
    }
    .badge-success { background: rgba(16, 185, 129, 0.15); color: var(--success); border: 1px solid rgba(16, 185, 129, 0.3); }
    .badge-danger { background: rgba(239, 68, 68, 0.15); color: var(--danger); border: 1px solid rgba(239, 68, 68, 0.3); }
    .badge-warning { background: rgba(245, 158, 11, 0.15); color: var(--warning); border: 1px solid rgba(245, 158, 11, 0.3); }
    .badge-info { background: rgba(6, 182, 212, 0.15); color: var(--accent); border: 1px solid rgba(6, 182, 212, 0.3); }

    /* LOGS CONSOLE */
    .terminal {
      background: #060911;
      border: 1px solid var(--card-border);
      border-radius: 12px;
      padding: 1rem;
      font-family: var(--font-mono);
      font-size: 0.8rem;
      height: 480px;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
      gap: 0.4rem;
    }
    .log-line { display: flex; gap: 0.75rem; line-height: 1.4; }
    .log-time { color: #64748b; }
    .log-level { font-weight: 700; width: 55px; }
    .log-level.INFO { color: var(--accent); }
    .log-level.WARN { color: var(--warning); }
    .log-level.ERROR { color: var(--danger); }
    .log-msg { color: #e2e8f0; flex: 1; word-break: break-all; }

    /* CODE PREVIEW */
    .code-block {
      background: #060911;
      border: 1px solid var(--card-border);
      border-radius: 8px;
      padding: 1rem;
      font-family: var(--font-mono);
      font-size: 0.825rem;
      overflow-x: auto;
      color: #38bdf8;
    }
  </style>
</head>
<body>

  <!-- LOGIN MODAL SI NO ESTA AUTENTICADO -->
  <div id="loginOverlay">
    <div class="login-card">
      <div style="display:flex; justify-content:center; margin-bottom:1rem;">
        <div class="brand-icon" style="width:52px; height:52px;">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
        </div>
      </div>
      <h2 style="font-size:1.4rem; font-weight:800; margin-bottom:0.25rem;">Panel de Administración</h2>
      <p style="font-size:0.8rem; color:var(--text-muted); margin-bottom:1.5rem;">Inicia sesión con las credenciales preconfiguradas</p>
      
      <form id="loginForm" onsubmit="handleLogin(event)">
        <div class="input-group" style="text-align:left;">
          <label>Usuario</label>
          <input type="text" id="loginUser" value="Administrator" required>
        </div>
        <div class="input-group" style="text-align:left;">
          <label>Contraseña</label>
          <input type="password" id="loginPass" value="admin12345" required>
        </div>
        <div id="loginError" style="display:none; color:var(--danger); font-size:0.8rem; margin-bottom:1rem; font-weight:600;">
          Credenciales incorrectas.
        </div>
        <button type="submit" class="btn" style="width:100%; justify-content:center; padding:0.75rem;">
          Ingresar al Panel
        </button>
      </form>
    </div>
  </div>

  <header>
    <div class="brand">
      <div class="brand-icon">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>
        </svg>
      </div>
      <div>
        <div style="display:flex; align-items:center; gap:0.5rem;">
          <span class="brand-title">Email Validator</span>
          <span class="brand-badge">PROD API</span>
        </div>
        <p style="font-size:0.75rem; color:var(--text-muted);">Panel de Control, Métricas & API Keys</p>
      </div>
    </div>

    <nav class="nav-tabs">
      <button class="tab-btn active" onclick="switchTab('metrics')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 3v18h18"/><path d="m19 9-5 5-4-4-3 3"/></svg>
        Métricas & KPIs
      </button>
      <button class="tab-btn" onclick="switchTab('keys')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/></svg>
        Gestión de API Keys
      </button>
      <button class="tab-btn" onclick="switchTab('swagger')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" x2="8" y1="13" y2="13"/><line x1="16" x2="8" y1="17" y2="17"/></svg>
        Swagger & Docs
      </button>
      <button class="tab-btn" onclick="switchTab('logs')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/></svg>
        Logs en Vivo
      </button>
    </nav>

    <div class="user-profile">
      <span class="badge badge-success" id="activeUserBadge">Administrator</span>
      <button class="btn btn-sm btn-danger" onclick="logout()">Salir</button>
    </div>
  </header>

  <main>
    <!-- TAB 1: METRICAS -->
    <section id="tab-metrics" class="tab-content active">
      <div class="metrics-grid">
        <div class="kpi-card">
          <div class="kpi-label">Peticiones Totales</div>
          <div class="kpi-value" id="kpi-total">0</div>
          <div class="kpi-sub">Tráfico global procesado</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">Tasa de Entregabilidad</div>
          <div class="kpi-value" style="color:var(--success)" id="kpi-deliverability">0%</div>
          <div class="kpi-sub"><span id="kpi-mx-count">0</span> con registros MX activos</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">Latencia Promedio</div>
          <div class="kpi-value" style="color:var(--accent)" id="kpi-latency">0 ms</div>
          <div class="kpi-sub">Tiempo de resolución DNS</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">Uptime Servidor</div>
          <div class="kpi-value" style="color:var(--primary)" id="kpi-uptime">0s</div>
          <div class="kpi-sub">Tiempo activo continuo</div>
        </div>
      </div>

      <div style="display:grid; grid-template-columns: 2fr 1fr; gap:1.5rem;">
        <div class="card">
          <div class="card-header">
            <h3 class="card-title">Desglose de Resoluciones</h3>
            <button class="btn btn-sm" onclick="loadMetrics()">Refrescar</button>
          </div>
          <div style="display:flex; flex-direction:column; gap:1rem;">
            <div>
              <div style="display:flex; justify-content:space-between; margin-bottom:0.4rem; font-size:0.875rem;">
                <span>Correos Entregables (Sintaxis OK + MX Encontrados)</span>
                <span id="prog-deliverable-val" style="font-weight:700; color:var(--success)">0</span>
              </div>
              <div style="background:rgba(255,255,255,0.05); height:8px; border-radius:4px; overflow:hidden;">
                <div id="prog-deliverable-bar" style="background:var(--success); width:0%; height:100%;"></div>
              </div>
            </div>
            <div>
              <div style="display:flex; justify-content:space-between; margin-bottom:0.4rem; font-size:0.875rem;">
                <span>Sin Registros MX (No Entregables)</span>
                <span id="prog-nomx-val" style="font-weight:700; color:var(--warning)">0</span>
              </div>
              <div style="background:rgba(255,255,255,0.05); height:8px; border-radius:4px; overflow:hidden;">
                <div id="prog-nomx-bar" style="background:var(--warning); width:0%; height:100%;"></div>
              </div>
            </div>
            <div>
              <div style="display:flex; justify-content:space-between; margin-bottom:0.4rem; font-size:0.875rem;">
                <span>Errores de Sintaxis RFC 5322</span>
                <span id="prog-syntax-val" style="font-weight:700; color:var(--danger)">0</span>
              </div>
              <div style="background:rgba(255,255,255,0.05); height:8px; border-radius:4px; overflow:hidden;">
                <div id="prog-syntax-bar" style="background:var(--danger); width:0%; height:100%;"></div>
              </div>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="card-header">
            <h3 class="card-title">Seguridad & Tráfico</h3>
          </div>
          <div style="display:flex; flex-direction:column; gap:0.75rem;">
            <div style="display:flex; justify-content:space-between; padding:0.5rem 0; border-bottom:1px solid rgba(255,255,255,0.05);">
              <span style="color:var(--text-muted)">Fallos de Autenticación</span>
              <span id="kpi-auth-fails" style="font-weight:700; color:var(--danger)">0</span>
            </div>
            <div style="display:flex; justify-content:space-between; padding:0.5rem 0; border-bottom:1px solid rgba(255,255,255,0.05);">
              <span style="color:var(--text-muted)">Límite de Payload</span>
              <span style="font-weight:700;">2 KB (Activo)</span>
            </div>
            <div style="display:flex; justify-content:space-between; padding:0.5rem 0;">
              <span style="color:var(--text-muted)">Almacén Persistente</span>
              <span class="badge badge-info">data/keys.json</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- TAB 2: GESTION DE API KEYS CON EXPIRACION -->
    <section id="tab-keys" class="tab-content">
      <div style="display:grid; grid-template-columns: 1fr 2fr; gap:1.5rem;">
        <div class="card">
          <div class="card-header">
            <h3 class="card-title">Crear Nueva API Key</h3>
          </div>
          <form id="createKeyForm" onsubmit="handleCreateKey(event)">
            <div class="input-group">
              <label for="keyClientName">Nombre del Cliente / Usuario</label>
              <input type="text" id="keyClientName" placeholder="Ej: App Móvil, Integración Pagos..." required>
            </div>
            <div class="input-group">
              <label for="keyExpiration">Tiempo de Expiración</label>
              <select id="keyExpiration">
                <option value="0">♾️ Eterna (Sin Expiración)</option>
                <option value="7">7 Días</option>
                <option value="30">30 Días (1 Mes)</option>
                <option value="90">90 Días (3 Meses)</option>
                <option value="365">365 Días (1 Año)</option>
              </select>
            </div>
            <button type="submit" class="btn" style="width:100%;">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
              Generar API Key
            </button>
          </form>
          <div id="newKeyAlert" style="display:none; margin-top:1rem; padding:0.875rem; background:rgba(16,185,129,0.1); border:1px solid rgba(16,185,129,0.3); border-radius:8px;">
            <p style="font-size:0.75rem; color:var(--success); font-weight:700;">¡Clave Creada Exitosamente!</p>
            <div style="display:flex; gap:0.5rem; margin-top:0.4rem;">
              <input type="text" id="createdKeyVal" readonly style="font-family:var(--font-mono); font-size:0.8rem;">
              <button class="btn btn-sm" onclick="copyCreatedKey()">Copiar</button>
            </div>
          </div>
        </div>

        <div class="card">
          <div class="card-header">
            <h3 class="card-title">API Keys Registradas</h3>
            <button class="btn btn-sm" onclick="loadKeys()">Recargar</button>
          </div>
          <div class="table-responsive">
            <table>
              <thead>
                <tr>
                  <th>Cliente / Nombre</th>
                  <th>API Key</th>
                  <th>Estado</th>
                  <th>Expiración</th>
                  <th>Peticiones</th>
                  <th>Acción</th>
                </tr>
              </thead>
              <tbody id="keysTableBody">
                <tr><td colspan="6" style="text-align:center; color:var(--text-muted)">Cargando claves...</td></tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </section>

    <!-- TAB 3: SWAGGER / OPENAPI & TESTER -->
    <section id="tab-swagger" class="tab-content">
      <div style="display:grid; grid-template-columns: 1fr 1fr; gap:1.5rem;">
        <div class="card">
          <div class="card-header">
            <h3 class="card-title">🧪 Probador Interactivo (Try It Out)</h3>
          </div>
          <div class="input-group">
            <label>Seleccionar API Key Activa</label>
            <select id="testerKeySelect" onchange="syncTesterKey()"></select>
          </div>
          <div class="input-group">
            <label>Header X-API-Key</label>
            <input type="text" id="testerCustomKey" style="font-family:var(--font-mono)">
          </div>
          <div class="input-group">
            <label>Correo Electrónico a Validar</label>
            <input type="email" id="testerEmail" value="contacto@empresa.com">
          </div>
          <button class="btn" onclick="executeValidationTest()">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"/></svg>
            Validar Correo
          </button>
        </div>

        <div class="card">
          <div class="card-header">
            <h3 class="card-title">Respuesta JSON</h3>
            <span id="testerStatusBadge" class="badge">Listo</span>
          </div>
          <pre class="code-block" id="testerResponseBlock">{
  "status": "Esperando prueba..."
}</pre>
        </div>
      </div>

      <div class="card">
        <div class="card-header">
          <h3 class="card-title">📖 Especificación OpenAPI 3.0</h3>
          <a href="/api/v1/openapi.json" target="_blank" class="btn btn-sm">Ver OpenAPI JSON</a>
        </div>
        <div style="display:flex; flex-direction:column; gap:1rem; font-size:0.875rem;">
          <div style="background:rgba(255,255,255,0.02); padding:1rem; border-radius:8px; border-left:4px solid var(--primary)">
            <strong>POST /api/v1/validate-email</strong>
            <p style="color:var(--text-muted); margin:0.3rem 0;">Valida sintaxis RFC 5322 y resuelve registros MX DNS asíncronos en tiempo real con verificación de expiración de clave.</p>
            <div style="font-size:0.75rem; color:var(--text-muted)">Headers: <code>Content-Type: application/json</code>, <code>X-API-Key: string</code></div>
          </div>
          <div style="background:rgba(255,255,255,0.02); padding:1rem; border-radius:8px; border-left:4px solid var(--success)">
            <strong>GET /health</strong>
            <p style="color:var(--text-muted); margin:0.3rem 0;">Healthcheck público para Kubernetes, Docker y balanceadores de carga.</p>
          </div>
        </div>
      </div>
    </section>

    <!-- TAB 4: LOGS EN VIVO -->
    <section id="tab-logs" class="tab-content">
      <div class="card">
        <div class="card-header">
          <h3 class="card-title">Consola de Tracing en Vivo</h3>
          <div style="display:flex; gap:0.5rem;">
            <button class="btn btn-sm" onclick="loadLogs()">Recargar Logs</button>
            <button class="btn btn-sm btn-danger" onclick="clearLogsView()">Limpiar Vista</button>
          </div>
        </div>
        <div class="terminal" id="terminalContainer"></div>
      </div>
    </section>
  </main>

  <script>
    let currentTab = 'metrics';

    function getAuthHeaders() {
      const token = sessionStorage.getItem('admin_token');
      return token ? { 'Authorization': `Basic ${token}` } : {};
    }

    function checkAuth() {
      const token = sessionStorage.getItem('admin_token');
      const overlay = document.getElementById('loginOverlay');
      if (token) {
        overlay.style.display = 'none';
        loadMetrics();
      } else {
        overlay.style.display = 'flex';
      }
    }

    async function handleLogin(e) {
      e.preventDefault();
      const u = document.getElementById('loginUser').value.trim();
      const p = document.getElementById('loginPass').value.trim();
      const cred = btoa(`${u}:${p}`);

      const res = await fetch('/api/v1/admin/login', {
        headers: { 'Authorization': `Basic ${cred}` }
      });

      if (res.ok) {
        sessionStorage.setItem('admin_token', cred);
        document.getElementById('loginError').style.display = 'none';
        document.getElementById('loginOverlay').style.display = 'none';
        loadMetrics();
      } else {
        document.getElementById('loginError').style.display = 'block';
      }
    }

    function logout() {
      sessionStorage.removeItem('admin_token');
      location.reload();
    }

    function switchTab(tabId) {
      currentTab = tabId;
      document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
      
      event.currentTarget.classList.add('active');
      document.getElementById(`tab-${tabId}`).classList.add('active');

      if (tabId === 'metrics') loadMetrics();
      if (tabId === 'keys') loadKeys();
      if (tabId === 'swagger') loadKeysForTester();
      if (tabId === 'logs') loadLogs();
    }

    // CARGAR METRICAS
    async function loadMetrics() {
      try {
        const res = await fetch('/api/v1/admin/metrics', { headers: getAuthHeaders() });
        if (res.status === 401) { logout(); return; }
        if (!res.ok) return;
        const d = await res.json();

        document.getElementById('kpi-total').innerText = d.total_requests;
        document.getElementById('kpi-deliverability').innerText = d.deliverability_rate.toFixed(1) + '%';
        document.getElementById('kpi-mx-count').innerText = d.deliverable_count;
        document.getElementById('kpi-latency').innerText = d.average_latency_ms.toFixed(1) + ' ms';
        document.getElementById('kpi-auth-fails').innerText = d.auth_failures;

        // Uptime formatting
        const s = d.uptime_seconds;
        const m = Math.floor(s / 60);
        const h = Math.floor(m / 60);
        document.getElementById('kpi-uptime').innerText = h > 0 ? `${h}h ${m%60}m` : `${m}m ${s%60}s`;

        // Bars
        const total = d.successful_validations + d.invalid_syntax_count || 1;
        document.getElementById('prog-deliverable-val').innerText = d.deliverable_count;
        document.getElementById('prog-deliverable-bar').style.width = ((d.deliverable_count / total) * 100) + '%';

        document.getElementById('prog-nomx-val').innerText = d.non_deliverable_count;
        document.getElementById('prog-nomx-bar').style.width = ((d.non_deliverable_count / total) * 100) + '%';

        document.getElementById('prog-syntax-val').innerText = d.invalid_syntax_count;
        document.getElementById('prog-syntax-bar').style.width = ((d.invalid_syntax_count / total) * 100) + '%';
      } catch (e) {
        console.error('Error cargando métricas:', e);
      }
    }

    // CARGAR KEYS
    async function loadKeys() {
      try {
        const res = await fetch('/api/v1/admin/keys', { headers: getAuthHeaders() });
        if (!res.ok) return;
        const keys = await res.json();
        const tbody = document.getElementById('keysTableBody');
        tbody.innerHTML = '';

        const now = new Date();

        keys.forEach(k => {
          # Ocultar la clave maestra del .env - no mostrar su valor por seguridad
          if (k.id === 'master-key') {
            const tr = document.createElement('tr');
            tr.innerHTML = '''
              <td colspan="6" style="text-align:center; color:var(--text-muted); font-style:italic;">
                <strong>Clave Maestra</strong><br>
                <span style="font-size:0.7rem; color:var(--text-muted);">Configurada en .env - valor no mostrado por seguridad</span>
              </td>
            ''';
            tbody.appendChild(tr);
            return; // Skip further processing for master key
          }

          const tr = document.createElement('tr');
          const isMaster = k.id === 'master-key';

          # Expiration status
          let expHtml = '<span class="badge badge-info">♾️ Eterna</span>';
          let isExpired = false;
          if (k.expires_at) {
            const expDate = new Date(k.expires_at);
            if (now > expDate) {
              isExpired = true;
              expHtml = `<span class="badge badge-danger">Expirada (${expDate.toLocaleDateString()})</span>`;
            } else {
              const diffDays = Math.ceil((expDate - now) / (1000 * 60 * 60 * 24));
              expHtml = `<span class="badge badge-warning">${diffDays} días restantes</span>`;
            }
          }

          let statusHtml = k.is_active 
            ? (isExpired ? '<span class="badge badge-danger">Inactiva (Expirada)</span>' : '<span class="badge badge-success">Activa</span>')
            : '<span class="badge badge-danger">Desactivada</span>';

          tr.innerHTML = `
            <td>
              <strong>${k.name}</strong><br>
              <span style="font-size:0.7rem; color:var(--text-muted); font-family:var(--font-mono)">${k.id}</span>
            </td>
            <td>
              <code style="font-family:var(--font-mono); color:var(--accent);">${k.key.substring(0, 10)}...</code>
              <button class="btn btn-sm" style="padding:0.2rem 0.5rem; margin-left:0.4rem;" onclick="copyText('${k.key}')">Copiar</button>
            </td>
            <td>${statusHtml}</td>
            <td>${expHtml}</td>
            <td><strong style="font-family:var(--font-mono)">${k.total_requests}</strong></td>
            <td>
              ${isMaster ? '<span style="font-size:0.75rem; color:var(--text-muted)">Master</span>' : `
                <button class="btn btn-sm" onclick="toggleKey('${k.id}')">${k.is_active ? 'Desactivar' : 'Activar'}</button>
                <button class="btn btn-sm btn-danger" onclick="deleteKey('${k.id}')">Eliminar</button>
              `}
            </td>
          `;
          tbody.appendChild(tr);
        });
      } catch (e) {
        console.error('Error cargando keys:', e);
      }
    }

    async function handleCreateKey(e) {
      e.preventDefault();
      const name = document.getElementById('keyClientName').value.trim();
      const expDays = parseInt(document.getElementById('keyExpiration').value, 10);
      if (!name) return;

      const res = await fetch('/api/v1/admin/keys', {
        method: 'POST',
        headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          name,
          expires_in_days: expDays > 0 ? expDays : null 
        })
      });

      if (res.ok) {
        const data = await res.json();
        document.getElementById('keyClientName').value = '';
        document.getElementById('newKeyAlert').style.display = 'block';
        document.getElementById('createdKeyVal').value = data.key;
        loadKeys();
      }
    }

    async function toggleKey(id) {
      await fetch(`/api/v1/admin/keys/${id}/toggle`, { method: 'POST', headers: getAuthHeaders() });
      loadKeys();
    }

    async function deleteKey(id) {
      if (!confirm('¿Seguro que deseas eliminar esta API Key permanentemente?')) return;
      await fetch(`/api/v1/admin/keys/${id}`, { method: 'DELETE', headers: getAuthHeaders() });
      loadKeys();
    }

    function copyText(txt) {
      navigator.clipboard.writeText(txt);
      alert('¡Clave copiada al portapapeles!');
    }

    function copyCreatedKey() {
      const val = document.getElementById('createdKeyVal').value;
      copyText(val);
    }

    // SWAGGER / TESTER
    async function loadKeysForTester() {
      const res = await fetch('/api/v1/admin/keys', { headers: getAuthHeaders() });
      if (!res.ok) return;
      const keys = await res.json();
      const select = document.getElementById('testerKeySelect');
      select.innerHTML = '';
      keys.forEach(k => {
        # Skip master key from tester selector
        if (k.id === 'master-key') return;
        const opt = document.createElement('option');
        opt.value = k.key;
        opt.text = `${k.name} (${k.key.substring(0, 10)}...)`;
        select.appendChild(opt);
      });
      syncTesterKey();
    }

    function syncTesterKey() {
      const select = document.getElementById('testerKeySelect');
      document.getElementById('testerCustomKey').value = select.value || '';
    }

    async function executeValidationTest() {
      const key = document.getElementById('testerCustomKey').value.trim();
      const email = document.getElementById('testerEmail').value.trim();
      const badge = document.getElementById('testerStatusBadge');
      const out = document.getElementById('testerResponseBlock');

      badge.className = 'badge badge-info';
      badge.innerText = 'Consultando...';

      try {
        const start = performance.now();
        const res = await fetch('/api/v1/validate-email', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-API-Key': key
          },
          body: JSON.stringify({ email })
        });
        const elapsed = (performance.now() - start).toFixed(0);
        const data = await res.json();

        if (res.ok) {
          badge.className = 'badge badge-success';
          badge.innerText = `HTTP ${res.status} OK (${elapsed}ms)`;
        } else {
          badge.className = 'badge badge-danger';
          badge.innerText = `HTTP ${res.status} Error`;
        }
        out.innerText = JSON.stringify(data, null, 2);
      } catch (err) {
        badge.className = 'badge badge-danger';
        badge.innerText = 'Network Error';
        out.innerText = String(err);
      }
    }

    // LOGS
    async function loadLogs() {
      try {
        const res = await fetch('/api/v1/admin/logs', { headers: getAuthHeaders() });
        if (!res.ok) return;
        const logs = await res.json();
        const container = document.getElementById('terminalContainer');
        container.innerHTML = '';

        logs.forEach(l => {
          const div = document.createElement('div');
          div.className = 'log-line';
          div.innerHTML = `
            <span class="log-time">${l.timestamp.split('T')[1].replace('Z','')}</span>
            <span class="log-level ${l.level}">${l.level}</span>
            <span class="log-msg">${l.message}</span>
          `;
          container.appendChild(div);
        });
        container.scrollTop = container.scrollHeight;
      } catch (e) {
        console.error('Error cargando logs:', e);
      }
    }

    function clearLogsView() {
      document.getElementById('terminalContainer').innerHTML = '';
    }

    // Auto-refresh métricas cada 5 segundos
    setInterval(() => {
      if (sessionStorage.getItem('admin_token')) {
        if (currentTab === 'metrics') loadMetrics();
        if (currentTab === 'logs') loadLogs();
      }
    }, 5000);

    // Comprobar autenticación inicial
    checkAuth();
  </script>
</body>
</html>
"#;
