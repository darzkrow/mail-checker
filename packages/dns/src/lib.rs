//! email-validator-dns - Resolución de registros MX DNS
use hickory_resolver::TokioAsyncResolver;
use shared::ValidationError;

/// Resolver de registros MX asíncrono
pub async fn resolve_mx_records(
    resolver: &TokioAsyncResolver,
    domain: &str,
) -> Result<bool, ValidationError> {
    match resolver.mx_lookup(domain).await {
        Ok(mx_lookup) => {
            let has_mx = mx_lookup.iter().count() > 0;
            Ok(has_mx)
        }
        Err(err) => {
            // Comprobar la causa del error en ResolveError
            use hickory_resolver::error::ResolveErrorKind;
            match err.kind() {
                ResolveErrorKind::NoRecordsFound { .. } => Ok(false),
                _ => {
                    let err_str = err.to_string();
                    if err_str.contains("no record found") || err_str.contains("NXDOMAIN") {
                        Ok(false)
                    } else {
                        Err(ValidationError::DnsError(format!(
                            "Error resolviendo registros MX para {}: {}",
                            domain, err
                        )))
                    }
                }
            }
        }
    }
}