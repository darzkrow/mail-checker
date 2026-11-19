//! Mock SMTP Server para testing local sin acceso a internet
//!
//! Simula respuestas SMTP típicas de Gmail, Outlook, catch-all domains, etc.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Tipo de respuesta simulada
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockSmtpBehavior {
    /// Gmail: entrega real para usuarios existentes
    Gmail,
    /// Outlook/Office365
    Outlook,
    /// Catch-all: acepta cualquier destinatario
    CatchAll,
    /// Greylisting: fallo temporal 450
    Greylisting,
    /// Todos rechazan (buzones inexistentes)
    AllReject,
    /// Timeout en conexión
    Timeout,
    /// Error TLS
    TlsError,
}

/// Configuración del mock server
#[derive(Debug, Clone)]
pub struct MockSmtpConfig {
    pub behavior: MockSmtpBehavior,
    pub valid_recipients: Vec<String>, // Para Gmail/Outlook: lista de buzones válidos
    pub delay_ms: u64, // Simular latencia
    pub require_starttls: bool,
}

impl Default for MockSmtpConfig {
    fn default() -> Self {
        Self {
            behavior: MockSmtpBehavior::Gmail,
            valid_recipients: vec![
                "test@gmail.com".to_string(),
                "usuario@gmail.com".to_string(),
                "admin@example.com".to_string(),
            ],
            delay_ms: 10,
            require_starttls: false,
        }
    }
}

/// Servidor SMTP Mock
pub struct MockSmtpServer {
    addr: SocketAddr,
    config: Arc<Mutex<MockSmtpConfig>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl MockSmtpServer {
    /// Crear y arrancar mock server
    pub async fn start(config: MockSmtpConfig) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let config = Arc::new(Mutex::new(config));

        info!("Mock SMTP server listening on {}", addr);

        let config_clone = config.clone();
        let handle = tokio::spawn(async move {
            Self::accept_loop(listener, config_clone).await;
        });

        Ok(Self {
            addr,
            config,
            _handle: handle,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Actualizar configuración en caliente
    pub async fn set_config(&self, config: MockSmtpConfig) {
        *self.config.lock().await = config;
    }

    async fn accept_loop(listener: TcpListener, config: Arc<Mutex<MockSmtpConfig>>) {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let config = config.clone();
                    tokio::spawn(Self::handle_connection(stream, peer_addr, config));
                }
                Err(e) => {
                    warn!("Error accepting connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        peer_addr: SocketAddr,
        config: Arc<Mutex<MockSmtpConfig>>,
    ) {
        debug!("New connection from {}", peer_addr);
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        let mut in_tls = false;
        let mut ehlo_done = false;
        let mut _mail_from = String::new();

        // Enviar banner inicial
        if Self::send_line(&mut reader, "220 mock-smtp.example.com ESMTP Ready").await.is_err() {
            return;
        }

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => {
                    debug!("Read error from {}: {}", peer_addr, e);
                    break;
                }
            }

            let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
            debug!("<< {}", trimmed);

            let (behavior, valid_recipients, delay, require_starttls) = {
                let cfg = config.lock().await;
                (cfg.behavior, cfg.valid_recipients.clone(), cfg.delay_ms, cfg.require_starttls)
            };

            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }

            let upper = trimmed.to_uppercase();

            if upper.starts_with("EHLO") || upper.starts_with("HELO") {
                ehlo_done = true;
                let mut response = String::from("250-mock-smtp.example.com Hello\r\n");
                response.push_str("250-SIZE 35882577\r\n");
                response.push_str("250-PIPELINING\r\n");
                response.push_str("250-SMTPUTF8\r\n");
                if !in_tls && require_starttls {
                    response.push_str("250-STARTTLS\r\n");
                } else {
                    // Última línea sin guión
                    response.push_str("250 OK\r\n");
                }
                if Self::send_multiline(&mut reader, &response).await.is_err() {
                    break;
                }
            } else if upper.starts_with("STARTTLS") {
                if require_starttls || behavior == MockSmtpBehavior::Outlook {
                    if Self::send_line(&mut reader, "220 2.0.0 Ready to start TLS").await.is_err() {
                        break;
                    }
                    // En un mock real, aquí haríamos upgrade a TLS
                    // Para simplificar, solo marcamos y continuamos en plano
                    in_tls = true;
                } else {
                    if Self::send_line(&mut reader, "454 4.7.0 STARTTLS not available").await.is_err() {
                        break;
                    }
                }
            } else if upper.starts_with("MAIL FROM:") {
                _mail_from = trimmed[10..].trim().trim_matches(|c| c == '<' || c == '>').to_string();
                if Self::send_line(&mut reader, "250 2.1.0 OK").await.is_err() {
                    break;
                }
            } else if upper.starts_with("RCPT TO:") {
                if !ehlo_done {
                    if Self::send_line(&mut reader, "503 5.5.1 EHLO/HELO first").await.is_err() {
                        break;
                    }
                    continue;
                }

                let recipient = trimmed[8..].trim().trim_matches(|c| c == '<' || c == '>').to_string();

                let response = match behavior {
                    MockSmtpBehavior::Gmail | MockSmtpBehavior::Outlook => {
                        if valid_recipients.iter().any(|v| v.eq_ignore_ascii_case(&recipient)) {
                            "250 2.1.5 OK"
                        } else {
                            "550 5.1.1 The email account that you tried to reach does not exist"
                        }
                    }
                    MockSmtpBehavior::CatchAll => "250 2.1.5 OK",
                    MockSmtpBehavior::Greylisting => "450 4.7.1 Temporary failure, please try again later",
                    MockSmtpBehavior::AllReject => "550 5.1.1 User unknown",
                    MockSmtpBehavior::Timeout => {
                        // No responder - simular timeout
                        continue;
                    }
                    MockSmtpBehavior::TlsError => "554 5.7.0 TLS required",
                };
                if Self::send_line(&mut reader, response).await.is_err() {
                    break;
                }
            } else if upper.starts_with("DATA") {
                if Self::send_line(&mut reader, "354 Go ahead").await.is_err() {
                    break;
                }
                // Leer hasta línea con solo "."
                loop {
                    line.clear();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                        break;
                    }
                    if line.trim() == "." {
                        break;
                    }
                }
                if Self::send_line(&mut reader, "250 2.0.0 OK").await.is_err() {
                    break;
                }
            } else if upper.starts_with("QUIT") {
                if Self::send_line(&mut reader, "221 2.0.0 Bye").await.is_err() {
                    break;
                }
                break;
            } else if upper.starts_with("RSET") {
                _mail_from.clear();
                if Self::send_line(&mut reader, "250 2.0.0 OK").await.is_err() {
                    break;
                }
            } else if upper.starts_with("NOOP") {
                if Self::send_line(&mut reader, "250 2.0.0 OK").await.is_err() {
                    break;
                }
            } else {
                if Self::send_line(&mut reader, "500 5.5.1 Command unrecognized").await.is_err() {
                    break;
                }
            }
        }

        debug!("Connection from {} closed", peer_addr);
    }

    async fn send_line(reader: &mut BufReader<TcpStream>, line: &str) -> Result<(), std::io::Error> {
        debug!(">> {}", line);
        reader.get_mut().write_all(format!("{}\r\n", line).as_bytes()).await?;
        reader.get_mut().flush().await
    }

    async fn send_multiline(reader: &mut BufReader<TcpStream>, response: &str) -> Result<(), std::io::Error> {
        for line in response.lines() {
            Self::send_line(reader, line).await?;
        }
        Ok(())
    }
}

/// Factory para crear mock servers predefinidos
pub struct MockSmtpFactory;

impl MockSmtpFactory {
    /// Gmail-like: solo usuarios en lista son válidos
    pub async fn gmail(valid: Vec<String>) -> MockSmtpServer {
        MockSmtpServer::start(MockSmtpConfig {
            behavior: MockSmtpBehavior::Gmail,
            valid_recipients: valid,
            delay_ms: 5,
            require_starttls: false,
        }).await.unwrap()
    }

    /// Outlook: requiere STARTTLS
    pub async fn outlook(valid: Vec<String>) -> MockSmtpServer {
        MockSmtpServer::start(MockSmtpConfig {
            behavior: MockSmtpBehavior::Outlook,
            valid_recipients: valid,
            delay_ms: 10,
            require_starttls: true,
        }).await.unwrap()
    }

    /// Catch-all: todo válido
    pub async fn catch_all() -> MockSmtpServer {
        MockSmtpServer::start(MockSmtpConfig {
            behavior: MockSmtpBehavior::CatchAll,
            valid_recipients: vec![],
            delay_ms: 5,
            require_starttls: false,
        }).await.unwrap()
    }

    /// Greylisting: fallo temporal
    pub async fn greylisting() -> MockSmtpServer {
        MockSmtpServer::start(MockSmtpConfig {
            behavior: MockSmtpBehavior::Greylisting,
            valid_recipients: vec![],
            delay_ms: 5,
            require_starttls: false,
        }).await.unwrap()
    }

    /// Todos rechazan
    pub async fn all_reject() -> MockSmtpServer {
        MockSmtpServer::start(MockSmtpConfig {
            behavior: MockSmtpBehavior::AllReject,
            valid_recipients: vec![],
            delay_ms: 5,
            require_starttls: false,
        }).await.unwrap()
    }
}

/// Helper para leer respuesta multilínea SMTP (público para tests de integración)
pub async fn read_smtp_response(reader: &mut BufReader<TcpStream>) -> String {
    let mut line = String::new();
    let mut response = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        response.push_str(&line);
        // Si la línea no empieza con código seguido de '-', es la última línea
        let trimmed = line.trim();
        if trimmed.len() >= 4 && &trimmed[3..4] != "-" {
            break;
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_gmail_deliverable() {
        let server = MockSmtpFactory::gmail(vec!["test@gmail.com".to_string()]).await;
        let port = server.port();

        let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let mut reader = BufReader::new(stream);

        // Banner
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("220"));

        // EHLO (multilínea)
        reader.get_mut().write_all(b"EHLO test\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        let resp = read_smtp_response(&mut reader).await;
        assert!(resp.starts_with("250"));

        // MAIL FROM
        reader.get_mut().write_all(b"MAIL FROM:<>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));

        // RCPT TO válido
        reader.get_mut().write_all(b"RCPT TO:<test@gmail.com>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));

        // QUIT
        reader.get_mut().write_all(b"QUIT\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_gmail_undeliverable() {
        let server = MockSmtpFactory::gmail(vec!["test@gmail.com".to_string()]).await;
        let port = server.port();

        let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let mut reader = BufReader::new(stream);

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap(); // banner
        eprintln!("Banner: {}", line.trim());

        // EHLO (multilínea)
        reader.get_mut().write_all(b"EHLO test\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        let resp = read_smtp_response(&mut reader).await;
        eprintln!("EHLO resp: {}", resp.trim());

        reader.get_mut().write_all(b"MAIL FROM:<>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear(); reader.read_line(&mut line).await.unwrap();
        eprintln!("MAIL FROM resp: {}", line.trim());
        assert!(line.starts_with("250"));

        reader.get_mut().write_all(b"RCPT TO:<noexiste@gmail.com>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear(); reader.read_line(&mut line).await.unwrap();
        eprintln!("RCPT TO resp: {}", line.trim());
        assert!(line.starts_with("550"));
        assert!(line.contains("5.1.1"));
    }

    #[tokio::test]
    async fn test_mock_catch_all() {
        let server = MockSmtpFactory::catch_all().await;
        let port = server.port();

        let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let mut reader = BufReader::new(stream);

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        // EHLO (multilínea)
        reader.get_mut().write_all(b"EHLO test\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        let _ = read_smtp_response(&mut reader).await;

        reader.get_mut().write_all(b"MAIL FROM:<>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear(); reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));

        // Cualquier destinatario es válido
        reader.get_mut().write_all(b"RCPT TO:<random-uuid-12345@catchall.com>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear(); reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));
    }

    #[tokio::test]
    async fn test_mock_greylisting() {
        let server = MockSmtpFactory::greylisting().await;
        let port = server.port();

        let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let mut reader = BufReader::new(stream);

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        // EHLO (multilínea)
        reader.get_mut().write_all(b"EHLO test\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        let _ = read_smtp_response(&mut reader).await;

        reader.get_mut().write_all(b"MAIL FROM:<>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear(); reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("250"));

        reader.get_mut().write_all(b"RCPT TO:<test@gmail.com>\r\n").await.unwrap();
        reader.get_mut().flush().await.unwrap();
        line.clear(); reader.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("450"));
    }
}