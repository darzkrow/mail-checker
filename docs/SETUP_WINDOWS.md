# 🪟 Guía de Instalación para Windows

Esta guía te ayudará a configurar y ejecutar el Email Validator API en Windows.

## 📋 **Prerrequisitos**

### **1. Instalar Docker Desktop**
1. Descarga Docker Desktop desde: https://www.docker.com/products/docker-desktop/
2. Ejecuta el instalador `Docker Desktop Installer.exe`
3. Sigue las instrucciones del asistente
4. Reinicia tu computadora cuando se solicite
5. Verifica la instalación:
   ```powershell
   docker --version
   ```

### **2. Configurar Docker (Opcional pero Recomendado)**
1. Abre Docker Desktop
2. Ve a Settings → Resources
3. Ajusta los recursos según tu sistema:
   - CPUs: 2-4
   - Memory: 4-8 GB
   - Swap: 1 GB

## 🚀 **Instalación Rápida**

### **Opción 1: Usar el Script Automático (Recomendado)**
```powershell
# 1. Navega al directorio del proyecto
cd C:\Users\Gfran\Desktop\Email-Validador

# 2. Ejecuta el script de instalación
.\run.bat
```

### **Opción 2: Configuración Manual**
```powershell
# 1. Crear archivo de configuración
copy .env.example .env

# 2. Editar el archivo .env (usar Notepad o VS Code)
notepad .env
```

**Configura el archivo `.env` con:**
```bash
# Cambia esta clave por una segura
API_KEY=mi-clave-secreta-windows-$(Get-Random -Count 32)

# Dominios permitidos (ajusta según necesites)
ALLOWED_ORIGINS=http://localhost:4000,http://localhost:8080

# Puerto (puedes cambiarlo si 3001 está en uso)
PORT=3001

# Rate limit (10 solicitudes/segundo por IP)
RATE_LIMIT_QPS=10

# Logging level
RUST_LOG=info
```

## 🏗️ **Construir y Ejecutar**

### **Con Docker Compose (Recomendado para Producción):**
```powershell
# Construir y ejecutar
docker-compose up --build

# Ejecutar en segundo plano
docker-compose up --build -d

# Ver logs
docker-compose logs -f

# Detener servicio
docker-compose down
```

### **Con Docker Directo:**
```powershell
# Construir imagen
docker build -t email-validator .

# Ejecutar contenedor
docker run -p 3001:3001 --env-file .env email-validator

# Ejecutar en segundo plano
docker run -d -p 3001:3001 --name email-validator-api --env-file .env email-validator
```

## 🧪 **Probar el Servicio**

### **Usar PowerShell:**
```powershell
# Definir variables
$API_KEY = (Get-Content .env | Select-String "API_KEY" | ForEach-Object { $_ -replace "API_KEY=", "" }).Trim()

# Probar el servicio
curl -X POST http://localhost:3001/api/v1/validate-email `
  -H "Content-Type: application/json" `
  -H "X-API-Key: $API_KEY" `
  -d '{"email":"test@gmail.com"}'
```

### **Usar Postman:**
1. Importa `docs/Email-Validator-API.postman_collection.json`
2. Configura las variables:
   - `base_url`: `http://localhost:3001`
   - `api_key`: (tu API_KEY del archivo .env)
3. Ejecuta las pruebas de la colección

## 🔧 **Solucionar Problemas Comunes**

### **Error: "Port already in use"**
```powershell
# Verificar qué proceso usa el puerto 3001
netstat -ano | findstr :3001

# Cambiar puerto en .env
# PORT=3001
```

### **Error: "Docker daemon not running"**
1. Abre Docker Desktop
2. Espera a que se inicie (icono en la bandeja)
3. Verifica con: `docker ps`

### **Error: "Permission denied"**
```powershell
# Ejecutar PowerShell como Administrador
# O ajustar permisos de Docker
```

### **Error: "Build failed"**
```powershell
# Limpiar cache de Docker
docker system prune -a

# Reintentar build
docker-compose build --no-cache
```

## 📊 **Monitoreo en Windows**

### **Ver logs del contenedor:**
```powershell
# Ver logs en tiempo real
docker logs -f email-validator-api

# Ver últimos 100 líneas
docker logs --tail 100 email-validator-api

# Ver logs con timestamp
docker logs -tf email-validator-api
```

### **Estadísticas del contenedor:**
```powershell
# Ver uso de recursos
docker stats email-validator-api

# Ver información del contenedor
docker inspect email-validator-api

# Ver procesos dentro del contenedor
docker top email-validator-api
```

## 🐳 **Comandos Docker Útiles**

### **Gestión de Contenedores:**
```powershell
# Listar contenedores
docker ps
docker ps -a  # Todos los contenedores

# Iniciar/detener contenedor
docker start email-validator-api
docker stop email-validator-api
docker restart email-validator-api

# Eliminar contenedor
docker rm email-validator-api
docker rm -f email-validator-api  # Forzar eliminación
```

### **Gestión de Imágenes:**
```powershell
# Listar imágenes
docker images

# Eliminar imagen
docker rmi email-validator

# Limpiar recursos no usados
docker system prune
```

### **Networking:**
```powershell
# Ver redes
docker network ls

# Inspeccionar red
docker network inspect email-validator-net

# Conectar a contenedor ejecutándose
docker exec -it email-validator-api /bin/sh
```

## 🚀 **Despliegue en Producción (Windows Server)**

### **1. Instalar Docker en Windows Server:**
```powershell
# Windows Server 2016/2019/2022
Install-Module -Name DockerMsftProvider -Repository PSGallery -Force
Install-Package -Name docker -ProviderName DockerMsftProvider
Restart-Computer -Force
```

### **2. Configurar como Servicio:**
```powershell
# Crear servicio con Docker Compose
docker-compose up -d

# Verificar servicio
Get-Service *docker*

# Configurar inicio automático
Set-Service -Name docker -StartupType Automatic
```

### **3. Configurar Firewall:**
```powershell
# Permitir puerto 3001
New-NetFirewallRule -DisplayName "Email Validator API" `
  -Direction Inbound -Protocol TCP -LocalPort 3001 -Action Allow
```

### **4. Configurar Logging:**
```powershell
# Configurar driver de logs
docker run --log-driver=json-file --log-opt max-size=10m --log-opt max-file=3
```

## 🔒 **Seguridad en Windows**

### **1. Variables de Entorno Seguras:**
```powershell
# Crear clave segura
$secureKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 64 | ForEach-Object {[char]$_})
[System.Environment]::SetEnvironmentVariable('API_KEY', $secureKey, 'Machine')

# Usar en .env
"API_KEY=$secureKey" | Out-File -FilePath .env -Append
```

### **2. Permisos de Archivos:**
```powershell
# Restringir acceso a .env
icacls .env /inheritance:r /grant:r "Administrators:F" /grant:r "SYSTEM:F"
```

### **3. Docker Security:**
```powershell
# Ejecutar como usuario no-root
docker run --user=1000:1000

# Limitar recursos
docker run --memory=256m --cpus=1.0
```

## 📁 **Estructura de Archivos**

```
Email-Validador/
├── run.bat                    # Script principal Windows
├── .env.example              # Ejemplo de configuración
├── docker-compose.yml        # Orquestación Docker
├── Dockerfile                # Configuración Docker
├── docs/                     # Documentación completa
│   ├── ERROR_CODES.md        # Códigos de error
│   ├── API_REFERENCE.md      # Referencia API
│   ├── SECURITY.md           # Guía de seguridad
│   └── Postman collection
└── README.md                 # Documentación principal
```

## 📞 **Soporte Técnico**

### **Problemas Comunes:**

1. **Docker no inicia:**
   - Verificar que Hyper-V esté habilitado
   - Reiniciar computadora
   - Ejecutar "Troubleshoot" en Docker Desktop

2. **Puerto en uso:**
   ```powershell
   # Liberar puerto 3001
   netstat -ano | findstr :3001
   taskkill /PID <PID> /F
   ```

3. **Problemas de red:**
   ```powershell
   # Resetear red de Docker
   docker network prune
   docker-compose down
   docker-compose up
   ```

### **Recursos Adicionales:**
- [Documentación Docker para Windows](https://docs.docker.com/desktop/windows/)
- [Troubleshooting Docker Desktop](https://docs.docker.com/desktop/troubleshoot/overview/)
- [Windows Container Documentation](https://docs.microsoft.com/en-us/virtualization/windowscontainers/)

## ✅ **Verificación Final**

```powershell
# Verificar que todo funciona
docker --version
docker-compose --version
docker ps
curl http://localhost:3001/api/v1/validate-email -H "X-API-Key: $env:API_KEY"
```

Si todos los comandos funcionan, ¡el servicio está listo! 🎉

---

**Última actualización:** 15 de octubre de 2026  
**Sistema operativo probado:** Windows 10/11, Windows Server 2019/2022  
**Docker version:** 20.10+  
**Estado:** ✅ Listo para producción
