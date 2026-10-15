#!/bin/bash

# ============================================
# Email Validator API - Docker Runner (Linux/Mac)
# ============================================

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}Email Validator API - Docker Runner${NC}"
echo -e "${BLUE}============================================${NC}"
echo

# Verificar si Docker está instalado
if ! command -v docker &> /dev/null; then
    echo -e "${RED} ERROR: Docker no está instalado. Por favor instala Docker primero.${NC}"
    echo
    echo "Pasos de instalación:"
    echo "1. Linux: sudo apt-get install docker.io"
    echo "2. Mac: https://docs.docker.com/desktop/install/mac-install/"
    echo "3. Reinicia tu terminal"
    echo
    exit 1
fi

echo -e "${GREEN} Docker está instalado:${NC}"
docker --version

echo

# Crear archivo .env si no existe
if [ ! -f .env ]; then
    echo -e "${YELLOW} WARNING: Archivo .env no encontrado. Creando con valores por defecto...${NC}"
    cp .env.example .env
    echo -e "${YELLOW} Archivo .env creado. Por favor edita el archivo y configura:${NC}"
    echo "   1. Una API_KEY segura"
    echo "   2. Los dominios ALLOWED_ORIGINS"
    echo "   3. Otras configuraciones según necesidad"
    echo
    read -p "Presiona Enter para continuar..." 
fi

# Leer API_KEY del archivo .env
API_KEY=$(grep "^API_KEY=" .env | cut -d'=' -f2)

if [ "$API_KEY" == "your-super-secret-api-key-change-me-in-production" ]; then
    echo -e "${RED} ERROR: API_KEY no configurada. Por favor edita el archivo .env.${NC}"
    echo
    echo "Instrucciones:"
    echo "1. Abre el archivo .env"
    echo "2. Cambia 'your-super-secret-api-key-change-me-in-production'"
    echo "3. Pon una clave segura (ej: my-secret-key-12345)"
    echo
    echo "Para generar una clave segura:"
    echo "  openssl rand -base64 32"
    echo
    exit 1
fi

API_KEY_LENGTH=${#API_KEY}
API_KEY_PREVIEW="${API_KEY:0:10}..."
echo -e "${GREEN} API_KEY configurada${NC}"
echo -e "${GREEN} Longitud de API_KEY: ${API_KEY_PREVIEW} (total: ${API_KEY_LENGTH} caracteres)${NC}"

echo

# Construir la imagen
echo -e "${BLUE}[STEP 1/3] Construyendo imagen Docker...${NC}"
docker build -t email-validator .

if [ $? -ne 0 ]; then
    echo -e "${RED}  ERROR: Falló la construcción de la imagen Docker${NC}"
    exit 1
fi

echo -e "${GREEN} Imagen construida exitosamente${NC}"

echo

# Mostrar información del servicio
echo -e "${BLUE}[STEP 2/3] Configurando servicio...${NC}"
echo
echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}SERVICIO CONFIGURADO${NC}"
echo -e "${BLUE}============================================${NC}"
echo -e "Puerto: ${GREEN}3000${NC}"
echo -e "URL: ${GREEN}http://localhost:3000${NC}"
echo -e "Endpoint: ${GREEN}POST /api/v1/validate-email${NC}"
echo
echo -e "${YELLOW}Comandos de prueba:${NC}"
echo "curl -X POST http://localhost:3000/api/v1/validate-email \\"
echo "  -H \"Content-Type: application/json\" \\"
echo "  -H \"X-API-Key: ${API_KEY}\" \\"
echo "  -d '{\"email\":\"test@gmail.com\"}'"
echo -e "${BLUE}============================================${NC}"

echo

# Preguntar al usuario
echo -e "${BLUE}[STEP 3/3] ¿Cómo deseas ejecutar el servicio?${NC}"
echo
echo -e "1. ${GREEN}Docker Compose${NC} (recomendado para producción)"
echo -e "2. ${GREEN}Docker run simple${NC} (para desarrollo)"
echo -e "3. ${YELLOW}Solo construir la imagen${NC}"
echo -e "4. ${RED}Salir${NC}"
echo

read -p "Selecciona una opción (1-4): " choice

case $choice in
    1)
        echo
        echo -e "${GREEN}  Ejecutando con Docker Compose...${NC}"
        echo -e "${YELLOW} Usa Ctrl+C para detener el servicio${NC}"
        echo
        docker-compose up --build
        ;;
    2)
        echo
        echo -e "${GREEN}  Ejecutando con Docker run...${NC}"
        echo -e "${YELLOW} Usa Ctrl+C para detener el servicio${NC}"
        echo
        docker run --rm -p 3000:3000 --env-file .env email-validator
        ;;
    3)
        echo
        echo -e "${GREEN} Imagen construida exitosamente.${NC}"
        echo -e "${YELLOW}Para ejecutar manualmente:${NC}"
        echo "  docker run --rm -p 3000:3000 --env-file .env email-validator"
        echo
        ;;
    *)
        echo
        echo -e "${BLUE}  Saliendo...${NC}"
        echo
        ;;
esac
