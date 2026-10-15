@echo off
REM ============================================
REM Email Validator API - Docker Runner (Windows)
REM ============================================

echo.
echo Email Validator API - Docker Runner (Windows)
echo ============================================
echo.

REM Verificar si Docker está instalado
docker --version >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Docker no está instalado. Por favor instala Docker Desktop primero.
    echo.
    echo Pasos de instalación:
    echo 1. Descarga Docker Desktop: https://www.docker.com/products/docker-desktop/
    echo 2. Instala Docker Desktop
    echo 3. Reinicia tu computadora
    echo 4. Vuelve a ejecutar este script
    echo.
    pause
    exit /b 1
)

echo [OK] Docker está instalado: 
docker --version | findstr version

echo.

REM Crear archivo .env si no existe
if not exist .env (
    echo [WARNING] Archivo .env no encontrado. Creando con valores por defecto...
    copy .env.example .env >nul
    echo [INFO] Archivo .env creado. Por favor edita el archivo y configura:
    echo       1. Una API_KEY segura
    echo       2. Los dominios ALLOWED_ORIGINS
    echo       3. Otras configuraciones según necesidad
    echo.
    pause
)

REM Leer API_KEY del archivo .env
set API_KEY=
for /f "tokens=2 delims==" %%a in ('findstr "^API_KEY=" .env') do set API_KEY=%%a

if "%API_KEY%"=="your-super-secret-api-key-change-me-in-production" (
    echo [ERROR] API_KEY no configurada. Por favor edita el archivo .env.
    echo.
    echo Instrucciones:
    echo 1. Abre el archivo .env
    echo 2. Cambia "your-super-secret-api-key-change-me-in-production"
    echo 3. Pon una clave segura (ej: my-secret-key-12345)
    echo.
    echo Para generar una clave segura en PowerShell:
    echo   $key = -join ((65..90)+(97..122)+(48..57) ^| Get-Random -Count 32 ^| ForEach-Object{[char]$_})
    echo   echo API_KEY=%key%
    echo.
    pause
    exit /b 1
)

echo [OK] API_KEY configurada
echo [INFO] Longitud de API_KEY: %API_KEY:~0,10%... (total: %API_KEY:~0,1% caracteres)

echo.

REM Construir la imagen
echo [STEP 1/3] Construyendo imagen Docker...
docker build -t email-validator .

if %errorlevel% neq 0 (
    echo [ERROR] Falló la construcción de la imagen Docker
    pause
    exit /b 1
)

echo [OK] Imagen construida exitosamente

echo.

REM Mostrar información del servicio
echo [STEP 2/3] Configurando servicio...
echo.
echo ============================================
echo SERVICIO CONFIGURADO
echo ============================================
echo Puerto: 3000
echo URL: http://localhost:3000
echo Endpoint: POST /api/v1/validate-email
echo.
echo Comandos de prueba:
echo curl -X POST http://localhost:3000/api/v1/validate-email ^
echo   -H "Content-Type: application/json" ^
echo   -H "X-API-Key: %API_KEY%" ^
echo   -d "{\"email\":\"test@gmail.com\"}"
echo.
echo ============================================

echo.

REM Preguntar al usuario
echo [STEP 3/3] ¿Cómo deseas ejecutar el servicio?
echo.
echo 1. Docker Compose (recomendado para producción)
echo 2. Docker run simple (para desarrollo)
echo 3. Solo construir la imagen
echo 4. Salir
echo.

set /p choice="Selecciona una opción (1-4): "

if "%choice%"=="1" (
    echo.
    echo [INFO] Ejecutando con Docker Compose...
    echo [INFO] Usa Ctrl+C para detener el servicio
    echo.
    docker-compose up --build
) else if "%choice%"=="2" (
    echo.
    echo [INFO] Ejecutando con Docker run...
    echo [INFO] Usa Ctrl+C para detener el servicio
    echo.
    docker run --rm -p 3000:3000 --env-file .env email-validator
) else if "%choice%"=="3" (
    echo.
    echo [INFO] Imagen construida exitosamente.
    echo Para ejecutar manualmente:
    echo   docker run --rm -p 3000:3000 --env-file .env email-validator
    echo.
) else (
    echo.
    echo [INFO] Saliendo...
)

pause
