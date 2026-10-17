//! email-validator-dns - Resolución asíncrona de registros MX y fallback SMTP (RFC 5321 y RFC 7505)
use hickory_resolver::TokioAsyncResolver;
use shared::ValidationError;

/// Resultado de la resolución de servidores de correo para un dominio
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsResolutionResult {
    /// Servidor de correo encontrado (vía registro MX o fallback A/AAAA)
    ValidMx,
    /// Dominio sin servidores de correo configurados ni host resoluble
    NoRecords,
    /// Rechazo explícito por Null MX según RFC 7505
    NullMx,
}

/// Resolver registros MX para un dominio con soporte de Null MX (RFC 7505) y fallback A/AAAA (RFC 5321)
pub async fn resolve_mx_records(
    resolver: &TokioAsyncResolver,
    domain: &str,
) -> Result<DnsResolutionResult, ValidationError> {
    let clean_domain = domain.trim().trim_end_matches('.');
    if clean_domain.is_empty() {
        return Ok(DnsResolutionResult::NoRecords);
    }

    match resolver.mx_lookup(clean_domain).await {
        Ok(mx_lookup) => {
            let mut valid_mx_found = false;

            for mx in mx_lookup.iter() {
                let exchange_str = mx.exchange().to_string();
                let clean_exchange = exchange_str.trim().trim_end_matches('.');

                // RFC 7505: "A 'Null MX' No-Service Resource Record"
                // Un registro MX con preferencia 0 y exchange "." o vacío indica explícitamente
                // que el dominio NO acepta correo electrónico bajo ninguna circunstancia.
                if mx.preference() == 0 && (clean_exchange.is_empty() || clean_exchange == ".") {
                    return Ok(DnsResolutionResult::NullMx);
                }

                if !clean_exchange.is_empty() {
                    valid_mx_found = true;
                }
            }

            if valid_mx_found {
                Ok(DnsResolutionResult::ValidMx)
            } else {
                // Si la lista de MX está vacía, aplicar fallback SMTP RFC 5321 Sección 5.1
                if resolve_fallback_host(resolver, clean_domain).await? {
                    Ok(DnsResolutionResult::ValidMx)
                } else {
                    Ok(DnsResolutionResult::NoRecords)
                }
            }
        }
        Err(err) => {
            use hickory_resolver::error::ResolveErrorKind;
            match err.kind() {
                ResolveErrorKind::NoRecordsFound { .. } => {
                    // Sin registros MX -> Fallback RFC 5321 a registros A/AAAA
                    if resolve_fallback_host(resolver, clean_domain).await? {
                        Ok(DnsResolutionResult::ValidMx)
                    } else {
                        Ok(DnsResolutionResult::NoRecords)
                    }
                }
                _ => {
                    let err_str = err.to_string();
                    if err_str.contains("no record found") || err_str.contains("NXDOMAIN") {
                        Ok(DnsResolutionResult::NoRecords)
                    } else {
                        Err(ValidationError::DnsError(format!(
                            "Error en la consulta DNS para el dominio '{}': {}",
                            clean_domain, err
                        )))
                    }
                }
            }
        }
    }
}

/// Fallback a registros A / AAAA según RFC 5321 Sección 5.1
/// Si un dominio carece de registros MX, se asume un registro MX implícito hacia el registro A/AAAA del host.
async fn resolve_fallback_host(
    resolver: &TokioAsyncResolver,
    domain: &str,
) -> Result<bool, ValidationError> {
    match resolver.lookup_ip(domain).await {
        Ok(ip_lookup) => {
            let has_ip = ip_lookup.iter().count() > 0;
            Ok(has_ip)
        }
        Err(_) => Ok(false),
    }
}