//! email-validator-smtp - Verificación real de entregabilidad SMTP mediante RCPT TO
//!
//! Este módulo implementa la verificación real de si un correo electrónico puede ser entregado,
//! conectándose a los servidores MX del dominio y ejecutando la transacción SMTP:
//! CONNECT → EHLO → STARTTLS (opcional) → MAIL FROM:<> → RCPT TO:<email> → QUIT

use std::time::Duration;
use shared::ValidationError;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

/// Resultado de la verificación SMTP
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SmtpVerificationResult {
    /// El buzón existe y acepta correo (código 250)
    Deliverable,
    /// El buzón no existe (códigos 550 5.1.x)
    Undeliverable,
    /// El buzón está lleno (códigos 552 5.2.x)
    MailboxFull,
    /// Rechazo por política/spam/reputación (códigos 554 5.7.x)
    RejectedByPolicy,
    /// Fallo temporal - greylisting, servidor ocupado (códigos 450, 451, 452)
    TemporaryFailure,
    /// Dominio catch-all - acepta cualquier destinatario
    CatchAll,
    /// Error de conexión/timeout/TLS
    ConnectionError,
    /// Error en resolución DNS previa (sin MX)
    NoMxRecords,
    /// Null MX explícito (RFC 7505)
    NullMx,
}

impl SmtpVerificationResult {
    /// ¿Indica que el correo es entregable de forma confiable?
    pub fn is_deliverable(&self) -> bool {
        matches!(self, SmtpVerificationResult::Deliverable)
    }

    /// ¿Es un error temporal que podría resolverse reintentando?
    pub fn is_temporary(&self) -> bool {
        matches!(self, SmtpVerificationResult::TemporaryFailure | SmtpVerificationResult::ConnectionError)
    }

    /// ¿Es un error permanente (no reintentar)?
    pub fn is_permanent_failure(&self) -> bool {
        matches!(
            self,
            SmtpVerificationResult::Undeliverable
                | SmtpVerificationResult::MailboxFull
                | SmtpVerificationResult::RejectedByPolicy
                | SmtpVerificationResult::NoMxRecords
                | SmtpVerificationResult::NullMx
        )
    }

    /// Código numérico para mapeo en API
    pub fn error_code(&self) -> u32 {
        match self {
            SmtpVerificationResult::Deliverable => 0,           // ERR_SUCCESS
            SmtpVerificationResult::Undeliverable => 9,         // ERR_SMTP_REJECTED_MAILBOX
            SmtpVerificationResult::MailboxFull => 9,           // ERR_SMTP_REJECTED_MAILBOX
            SmtpVerificationResult::RejectedByPolicy => 9,      // ERR_SMTP_REJECTED_POLICY
            SmtpVerificationResult::TemporaryFailure => 11,     // ERR_SMTP_TEMPORARY_FAILURE
            SmtpVerificationResult::CatchAll => 10,             // ERR_CATCH_ALL
            SmtpVerificationResult::ConnectionError => 12,      // ERR_SMTP_CONNECTION_ERROR
            SmtpVerificationResult::NoMxRecords => 3,           // ERR_NO_MX_RECORDS
            SmtpVerificationResult::NullMx => 4,                // ERR_NULL_MX_REJECTED
        }
    }

    /// Descripción legible para respuesta API
    pub fn reason(&self) -> &'static str {
        match self {
            SmtpVerificationResult::Deliverable => "Correo válido y buzón confirmado entregable (SMTP RCPT TO 250)",
            SmtpVerificationResult::Undeliverable => "Buzón inexistente (SMTP 550 5.1.1)",
            SmtpVerificationResult::MailboxFull => "Buzón lleno (SMTP 552 5.2.2)",
            SmtpVerificationResult::RejectedByPolicy => "Rechazado por política del servidor (SMTP 554 5.7.1)",
            SmtpVerificationResult::TemporaryFailure => "Fallo temporal - greylisting o servidor ocupado (SMTP 450/451/452)",
            SmtpVerificationResult::CatchAll => "Dominio catch-all - acepta cualquier destinatario (no verificable individualmente)",
            SmtpVerificationResult::ConnectionError => "Error de conexión/timeout al servidor SMTP",
            SmtpVerificationResult::NoMxRecords => "Dominio sin servidores de correo (sin registros MX)",
            SmtpVerificationResult::NullMx => "Dominio declara explícitamente no aceptar correo (Null MX RFC 7505)",
        }
    }
}

/// Configuración del verificador SMTP
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// Timeout total para la verificación (conexión + comandos)
    pub total_timeout: Duration,
    /// Timeout solo para conexión TCP
    pub connect_timeout: Duration,
    /// Timeout para cada comando SMTP individual
    pub command_timeout: Duration,
    /// Máximo número de reintentos en fallos temporales (4xx)
    pub max_retries: u32,
    /// Backoff base entre reintentos (se multiplica exponencialmente)
    pub retry_backoff_base: Duration,
    /// Dominio para enviar en EHLO
    pub helo_domain: String,
    /// Remitente para MAIL FROM (vacío = null sender <>)
    pub mail_from: String,
    /// Puerto SMTP principal
    pub primary_port: u16,
    /// Puerto submission (STARTTLS)
    pub submission_port: u16,
    /// Puerto SMTPS implícito
    pub smtps_port: u16,
    /// Aceptar certificados TLS inválidos (solo testing)
    pub danger_accept_invalid_certs: bool,
    /// Habilitar verificación catch-all
    pub enable_catch_all_detection: bool,
    /// Dominio de prueba para catch-all (se genera UUID aleatorio si None)
    pub catch_all_test_domain: Option<String>,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            total_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            command_timeout: Duration::from_secs(10),
            max_retries: 2,
            retry_backoff_base: Duration::from_secs(5),
            helo_domain: "email-validator.local".to_string(),
            mail_from: "".to_string(), // null sender <>
            primary_port: 25,
            submission_port: 587,
            smtps_port: 465,
            danger_accept_invalid_certs: false,
            enable_catch_all_detection: true,
            catch_all_test_domain: None,
        }
    }
}

/// Registro MX con prioridad para ordenamiento
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MxRecord {
    pub host: String,
    pub priority: u16,
    pub port: u16, // Puerto a usar (25, 587, 465)
    pub use_tls: bool, // true = STARTTLS en 587, implícito en 465
}

impl MxRecord {
    /// Crear registro MX para puerto 25 (sin TLS)
    pub fn new_port25(host: String, priority: u16) -> Self {
        Self {
            host,
            priority,
            port: 25,
            use_tls: false,
        }
    }

    /// Crear registro MX para puerto 587 con STARTTLS
    pub fn new_port587(host: String, priority: u16) -> Self {
        Self {
            host,
            priority,
            port: 587,
            use_tls: true,
        }
    }

    /// Crear registro MX para puerto 465 (SMTPS implícito)
    pub fn new_port465(host: String, priority: u16) -> Self {
        Self {
            host,
            priority,
            port: 465,
            use_tls: true,
        }
    }
}

/// Errores específicos del verificador SMTP
#[derive(Error, Debug)]
pub enum SmtpError {
    #[error("Error de conexión: {0}")]
    Connection(String),
    #[error("Timeout: {0}")]
    Timeout(String),
    #[error("Error TLS: {0}")]
    Tls(String),
    #[error("Error de protocolo SMTP: {0}")]
    Protocol(String),
    #[error("Respuesta inesperada: código {code}, mensaje: {message}")]
    UnexpectedResponse { code: u16, message: String },
    #[error("Sin servidores MX disponibles")]
    NoMxRecords,
    #[error("Null MX detectado (RFC 7505)")]
    NullMx,
    #[error("Error interno: {0}")]
    Internal(String),
}

impl From<SmtpError> for ValidationError {
    fn from(err: SmtpError) -> Self {
        ValidationError::DnsError(err.to_string())
    }
}

/// Respuesta SMTP parseada
#[derive(Debug, Clone)]
pub struct SmtpResponse {
    pub code: u16,
    pub message: String,
    pub is_continuation: bool, // true si código seguido de '-' (línea continua)
}

/// Cliente SMTP para verificación de entregabilidad
pub struct SmtpVerifier {
    config: SmtpConfig,
    tls_connector: TlsConnector,
}

impl SmtpVerifier {
    /// Obtener referencia a la configuración (para testing)
    pub fn config(&self) -> &SmtpConfig {
        &self.config
    }

    /// Crear nuevo verificador con configuración
    pub fn new(config: SmtpConfig) -> Result<Self, SmtpError> {
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let mut client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        if config.danger_accept_invalid_certs {
            client_config.dangerous().set_certificate_verifier(
                std::sync::Arc::new(NoCertificateVerification {}),
            );
        }

        let tls_connector = TlsConnector::from(std::sync::Arc::new(client_config));

        Ok(Self {
            config,
            tls_connector,
        })
    }

    /// Verificar entregabilidad de un email
    ///
    /// Flujo:
    /// 1. Resolver registros MX (usando dns::resolve_mx_records)
    /// 2. Para cada MX ordenado por prioridad:
    ///    a. Conectar (puerto 25 → 587 STARTTLS → 465)
    ///    b. EHLO
    ///    c. STARTTLS si disponible y puerto 587
    ///    d. MAIL FROM: <>
    ///    e. RCPT TO: <email>
    ///    f. QUIT
    /// 3. Retornar primer resultado exitoso o mejor error
    pub async fn verify(
        &self,
        resolver: &hickory_resolver::TokioAsyncResolver,
        email: &str,
    ) -> Result<SmtpVerificationResult, SmtpError> {
        // Extraer dominio
        let domain = email.split('@').nth(1).ok_or(SmtpError::Protocol("Email sin dominio".into()))?;

        // 1. Resolver MX records
        let mx_records = self.resolve_mx_records(resolver, domain).await?;

        if mx_records.is_empty() {
            return Ok(SmtpVerificationResult::NoMxRecords);
        }

        // 2. Detectar catch-all si habilitado
        if self.config.enable_catch_all_detection {
            if let Ok(is_catch_all) = self.test_catch_all(resolver, domain, &mx_records).await {
                if is_catch_all {
                    return Ok(SmtpVerificationResult::CatchAll);
                }
            }
        }

        // 3. Verificar email real contra cada MX
        let mut last_error = None;

        for mx in mx_records {
            match self.verify_against_mx(&mx, email).await {
                Ok(result) => {
                    // Si es entregable o error permanente, retornar inmediatamente
                    if result.is_deliverable() || result.is_permanent_failure() {
                        return Ok(result);
                    }
                    // Si es temporal, guardar y continuar con siguiente MX
                    last_error = Some(result);
                }
                Err(e) => {
                    last_error = Some(SmtpVerificationResult::ConnectionError);
                    tracing::warn!("Error verificando contra MX {}: {}", mx.host, e);
                    continue;
                }
            }
        }

        // Si llegamos aquí, todos los MX fallaron con errores temporales o conexión
        Ok(last_error.unwrap_or(SmtpVerificationResult::ConnectionError))
    }

    /// Resolver y ordenar registros MX con puertos de fallback
    async fn resolve_mx_records(
        &self,
        resolver: &hickory_resolver::TokioAsyncResolver,
        domain: &str,
    ) -> Result<Vec<MxRecord>, SmtpError> {
        use hickory_resolver::error::ResolveErrorKind;

        let clean_domain = domain.trim().trim_end_matches('.');

        let mx_lookup = match timeout(self.config.connect_timeout, resolver.mx_lookup(clean_domain)).await {
            Ok(Ok(lookup)) => lookup,
            Ok(Err(err)) => {
                match err.kind() {
                    ResolveErrorKind::NoRecordsFound { .. } => return Ok(vec![]),
                    _ => return Err(SmtpError::Connection(format!("Error DNS: {}", err))),
                }
            }
            Err(_) => return Err(SmtpError::Timeout("Timeout resolviendo MX".into())),
        };

        let mut records = Vec::new();

        // hickory-resolver 0.24: mx_lookup.iter() devuelve un iterador sobre MxRecord
        for mx in mx_lookup.iter() {
            let exchange = mx.exchange().to_string().trim_end_matches('.').to_string();
            let priority = mx.preference();

            // RFC 7505: Null MX (preferencia 0, exchange ".")
            if priority == 0 && (exchange.is_empty() || exchange == ".") {
                return Err(SmtpError::NullMx);
            }

            if !exchange.is_empty() {
                // Agregar con tres opciones de puerto (se intentarán en orden)
                records.push(MxRecord::new_port25(exchange.clone(), priority));
                records.push(MxRecord::new_port587(exchange.clone(), priority));
                records.push(MxRecord::new_port465(exchange, priority));
            }
        }

        // Ordenar por prioridad (menor = más prioritario), luego por puerto preferido (25, 587, 465)
        records.sort_by(|a, b| {
            a.priority.cmp(&b.priority).then_with(|| a.port.cmp(&b.port))
        });

        Ok(records)
    }

    /// Probar si el dominio es catch-all
    async fn test_catch_all(
        &self,
        _resolver: &hickory_resolver::TokioAsyncResolver,
        domain: &str,
        mx_records: &[MxRecord],
    ) -> Result<bool, SmtpError> {
        use rand::Rng;
        let test_local: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        let test_email = format!("{}@{}", test_local, domain);

        // Usar solo el primer MX de mayor prioridad para test catch-all
        if let Some(mx) = mx_records.first() {
            match self.verify_against_mx(mx, &test_email).await {
                Ok(SmtpVerificationResult::Deliverable) => Ok(true), // Catch-all detectado
                Ok(_) => Ok(false),
                Err(_) => Ok(false), // Si falla conexión, asumir no catch-all
            }
        } else {
            Ok(false)
        }
    }

    /// Verificar email contra un MX específico
    async fn verify_against_mx(
        &self,
        mx: &MxRecord,
        email: &str,
    ) -> Result<SmtpVerificationResult, SmtpError> {
        let stream = self.connect_mx(mx).await?;

        let result = self.smtp_transaction(stream, mx, email).await;

        result
    }

    /// Conectar a servidor MX (con TLS si corresponde)
    async fn connect_mx(&self, mx: &MxRecord) -> Result<Box<dyn AsyncReadWrite + Send + Unpin>, SmtpError> {
        let addr = format!("{}:{}", mx.host, mx.port);

        let stream = timeout(self.config.connect_timeout, TcpStream::connect(&addr))
            .await
            .map_err(|_| SmtpError::Timeout(format!("Timeout conectando a {}", addr)))?
            .map_err(|e| SmtpError::Connection(format!("Error conectando a {}: {}", addr, e)))?;

        // Configurar keepalive y nodelay
        stream.set_nodelay(true).ok();

        if mx.use_tls && mx.port == 587 {
            // Puerto 587: Conectar plano, luego STARTTLS
            return Ok(Box::new(PlainStream(stream)));
        } else if mx.use_tls && mx.port == 465 {
            // Puerto 465: SMTPS implícito - TLS desde el inicio
            let domain = rustls::pki_types::ServerName::try_from(mx.host.as_str())
                .map_err(|e| SmtpError::Tls(format!("Invalid SNI: {}", e)))?
                .to_owned();
            let tls_stream = timeout(self.config.connect_timeout, self.tls_connector.connect(domain, stream))
                .await
                .map_err(|_| SmtpError::Timeout("Timeout handshake TLS".into()))?
                .map_err(|e| SmtpError::Tls(format!("Error TLS: {}", e)))?;
            return Ok(Box::new(TlsStream(tls_stream)));
        }

        // Puerto 25: Sin TLS
        Ok(Box::new(PlainStream(stream)))
    }

    /// Ejecutar transacción SMTP completa
    async fn smtp_transaction(
        &self,
        mut stream: Box<dyn AsyncReadWrite + Send + Unpin>,
        mx: &MxRecord,
        email: &str,
    ) -> Result<SmtpVerificationResult, SmtpError> {
        // Si puerto 587, hacer STARTTLS después de EHLO
        if mx.port == 587 && mx.use_tls {
            return Box::pin(self.smtp_transaction_with_starttls(stream, mx, email)).await;
        }

        // Leer banner inicial (220)
        let banner = self.read_response(&mut stream).await?;
        if banner.code != 220 {
            return Err(SmtpError::UnexpectedResponse {
                code: banner.code,
                message: format!("Banner inesperado: {}", banner.message),
            });
        }

        // EHLO
        self.send_command(&mut stream, &format!("EHLO {}", self.config.helo_domain)).await?;
        let ehlo_resp = self.read_multiline_response(&mut stream).await?;
        if ehlo_resp.code != 250 {
            return Err(SmtpError::UnexpectedResponse {
                code: ehlo_resp.code,
                message: format!("EHLO falló: {}", ehlo_resp.message),
            });
        }

        // MAIL FROM
        let mail_from = if self.config.mail_from.is_empty() {
            "<>".to_string()
        } else {
            format!("<{}>", self.config.mail_from)
        };
        self.send_command(&mut stream, &format!("MAIL FROM:{}", mail_from)).await?;
        let mail_resp = self.read_response(&mut stream).await?;
        if mail_resp.code != 250 {
            return Err(SmtpError::UnexpectedResponse {
                code: mail_resp.code,
                message: format!("MAIL FROM falló: {}", mail_resp.message),
            });
        }

        // RCPT TO
        self.send_command(&mut stream, &format!("RCPT TO:<{}>", email)).await?;
        let rcpt_resp = self.read_response(&mut stream).await?;

        // QUIT
        let _ = self.send_command(&mut stream, "QUIT").await;
        let _ = self.read_response(&mut stream).await;

        // Interpretar respuesta RCPT TO
        self.parse_rcpt_response(rcpt_resp.code, &rcpt_resp.message)
    }

    /// Transacción SMTP con STARTTLS (puerto 587)
    async fn smtp_transaction_with_starttls(
        &self,
        mut stream: Box<dyn AsyncReadWrite + Send + Unpin>,
        mx: &MxRecord,
        email: &str,
    ) -> Result<SmtpVerificationResult, SmtpError> {
        // Leer banner
        let banner = self.read_response(&mut stream).await?;
        if banner.code != 220 {
            return Err(SmtpError::UnexpectedResponse {
                code: banner.code,
                message: format!("Banner inesperado: {}", banner.message),
            });
        }

        // EHLO inicial
        self.send_command(&mut stream, &format!("EHLO {}", self.config.helo_domain)).await?;
        let ehlo_resp = self.read_multiline_response(&mut stream).await?;
        if ehlo_resp.code != 250 {
            return Err(SmtpError::UnexpectedResponse {
                code: ehlo_resp.code,
                message: format!("EHLO falló: {}", ehlo_resp.message),
            });
        }

        // Verificar soporte STARTTLS
        let supports_starttls = ehlo_resp.message.to_uppercase().contains("STARTTLS");
        if !supports_starttls {
            return Err(SmtpError::Protocol("Servidor no soporta STARTTLS en puerto 587".into()));
        }

        // STARTTLS
        self.send_command(&mut stream, "STARTTLS").await?;
        let starttls_resp = self.read_response(&mut stream).await?;
        if starttls_resp.code != 220 {
            return Err(SmtpError::UnexpectedResponse {
                code: starttls_resp.code,
                message: format!("STARTTLS rechazado: {}", starttls_resp.message),
            });
        }

        // Cerrar conexión actual
        let _ = self.send_command(&mut stream, "QUIT").await;

        // Reconectar con TLS implícito (tratar puerto 587 como 465 después de STARTTLS)
        let tls_stream = self.connect_mx(&MxRecord::new_port465(mx.host.clone(), mx.priority)).await?;

        // Continuar transacción en stream TLS
        self.smtp_transaction(tls_stream, mx, email).await
    }

    /// Enviar comando SMTP y flush
    async fn send_command(
        &self,
        stream: &mut (dyn AsyncReadWrite + Send + Unpin),
        command: &str,
    ) -> Result<(), SmtpError> {
        let cmd = format!("{}\r\n", command);
        timeout(self.config.command_timeout, stream.write_all(cmd.as_bytes()))
            .await
            .map_err(|_| SmtpError::Timeout(format!("Timeout enviando: {}", command)))?
            .map_err(|e| SmtpError::Connection(format!("Error escribiendo: {}", e)))?;

        timeout(self.config.command_timeout, stream.flush())
            .await
            .map_err(|_| SmtpError::Timeout("Timeout flush".into()))?
            .map_err(|e| SmtpError::Connection(format!("Error flush: {}", e)))?;

        Ok(())
    }

    /// Leer una línea de respuesta SMTP
    async fn read_response(
        &self,
        stream: &mut (dyn AsyncReadWrite + Send + Unpin),
    ) -> Result<SmtpResponse, SmtpError> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        timeout(self.config.command_timeout, reader.read_line(&mut line))
            .await
            .map_err(|_| SmtpError::Timeout("Timeout leyendo respuesta".into()))?
            .map_err(|e| SmtpError::Connection(format!("Error leyendo: {}", e)))?;

        let line = line.trim_end_matches(&['\r', '\n'][..]);

        // Parsear código (primeros 3 dígitos)
        let code = line[..3].parse::<u16>().unwrap_or(0);
        let message = if line.len() > 4 { line[4..].to_string() } else { String::new() };
        let is_continuation = line.len() >= 4 && line.as_bytes()[3] == b'-';

        Ok(SmtpResponse {
            code,
            message,
            is_continuation,
        })
    }

    /// Leer respuesta multilínea (códigos con '-')
    async fn read_multiline_response(
        &self,
        stream: &mut (dyn AsyncReadWrite + Send + Unpin),
    ) -> Result<SmtpResponse, SmtpError> {
        let mut last_message = String::new();

        loop {
            let resp = self.read_response(stream).await?;
            if !resp.message.is_empty() {
                if !last_message.is_empty() {
                    last_message.push(' ');
                }
                last_message.push_str(&resp.message);
            }

            if !resp.is_continuation {
                return Ok(SmtpResponse {
                    code: resp.code,
                    message: last_message,
                    is_continuation: false,
                });
            }
        }
    }

    /// Interpretar código de respuesta RCPT TO
    fn parse_rcpt_response(&self, code: u16, message: &str) -> Result<SmtpVerificationResult, SmtpError> {
        match code {
            250 | 251 | 252 => Ok(SmtpVerificationResult::Deliverable),
            550 => {
                // 550 5.1.1 - Usuario no existe
                // 550 5.1.0 - Dirección rechazada
                // 550 5.1.10 - Dirección no encontrada
                if message.contains("5.1.1") || message.contains("5.1.0") || message.contains("5.1.10")
                    || message.to_lowercase().contains("user unknown")
                    || message.to_lowercase().contains("no such user")
                    || message.to_lowercase().contains("mailbox not found")
                    || message.to_lowercase().contains("account not found")
                {
                    Ok(SmtpVerificationResult::Undeliverable)
                } else if message.contains("5.7.1") || message.to_lowercase().contains("policy")
                    || message.to_lowercase().contains("spam")
                    || message.to_lowercase().contains("blocked")
                    || message.to_lowercase().contains("rejected")
                {
                    Ok(SmtpVerificationResult::RejectedByPolicy)
                } else {
                    Ok(SmtpVerificationResult::Undeliverable)
                }
            }
            551 => Ok(SmtpVerificationResult::Undeliverable), // Usuario no local
            552 => {
                // 552 5.2.2 - Buzón lleno
                // 552 5.2.3 - Límite de mensajes excedido
                if message.contains("5.2.2") || message.contains("5.2.3")
                    || message.to_lowercase().contains("mailbox full")
                    || message.to_lowercase().contains("quota exceeded")
                    || message.to_lowercase().contains("over quota")
                {
                    Ok(SmtpVerificationResult::MailboxFull)
                } else {
                    Ok(SmtpVerificationResult::Undeliverable)
                }
            }
            553 => Ok(SmtpVerificationResult::Undeliverable), // Nombre de buzón no válido
            554 => {
                // 554 5.7.1 - Rechazo por política
                if message.contains("5.7.1") || message.to_lowercase().contains("policy")
                    || message.to_lowercase().contains("spam")
                    || message.to_lowercase().contains("blocked")
                    || message.to_lowercase().contains("rejected")
                    || message.to_lowercase().contains("blacklist")
                {
                    Ok(SmtpVerificationResult::RejectedByPolicy)
                } else {
                    Ok(SmtpVerificationResult::Undeliverable)
                }
            }
            450 | 451 | 452 => Ok(SmtpVerificationResult::TemporaryFailure), // Greylisting, ocupado
            _ => Err(SmtpError::UnexpectedResponse {
                code,
                message: message.to_string(),
            }),
        }
    }
}

// Trait para abstraer streams (plano y TLS)
trait AsyncReadWrite: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> AsyncReadWrite for T {}

struct PlainStream(TcpStream);
impl tokio::io::AsyncRead for PlainStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
impl tokio::io::AsyncWrite for PlainStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        std::pin::Pin::new(&mut self.0).poll_write(cx, buf)
    }
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut self.0).poll_flush(cx)
    }
    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

struct TlsStream(tokio_rustls::client::TlsStream<TcpStream>);
impl tokio::io::AsyncRead for TlsStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
impl tokio::io::AsyncWrite for TlsStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        std::pin::Pin::new(&mut self.0).poll_write(cx, buf)
    }
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut self.0).poll_flush(cx)
    }
    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::pin::Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

/// Verificador de certificados que no valida (solo testing)
#[derive(Debug)]
struct NoCertificateVerification;
impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

// Cache y Connection Pool
pub mod cache;

// Mock server module for testing
pub mod mock_server;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_result_codes() {
        assert_eq!(SmtpVerificationResult::Deliverable.error_code(), 0);
        assert_eq!(SmtpVerificationResult::Undeliverable.error_code(), 9);
        assert_eq!(SmtpVerificationResult::CatchAll.error_code(), 10);
        assert_eq!(SmtpVerificationResult::TemporaryFailure.error_code(), 11);
        assert_eq!(SmtpVerificationResult::ConnectionError.error_code(), 12);
    }

    #[test]
    fn test_smtp_result_properties() {
        assert!(SmtpVerificationResult::Deliverable.is_deliverable());
        assert!(!SmtpVerificationResult::Undeliverable.is_deliverable());
        assert!(SmtpVerificationResult::TemporaryFailure.is_temporary());
        assert!(SmtpVerificationResult::Undeliverable.is_permanent_failure());
    }

    #[test]
    fn test_mx_record_creation() {
        let mx25 = MxRecord::new_port25("mail.example.com".into(), 10);
        assert_eq!(mx25.port, 25);
        assert!(!mx25.use_tls);

        let mx587 = MxRecord::new_port587("mail.example.com".into(), 10);
        assert_eq!(mx587.port, 587);
        assert!(mx587.use_tls);

        let mx465 = MxRecord::new_port465("mail.example.com".into(), 10);
        assert_eq!(mx465.port, 465);
        assert!(mx465.use_tls);
    }
}