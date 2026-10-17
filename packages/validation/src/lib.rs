//! email-validator-validation - Validación robusta de sintaxis de email (RFC 5321 y RFC 5322)
use shared::{EmailValidationResponse, ValidationError};

/// Caracteres especiales permitidos en la parte local según RFC 5322 (atext)
const VALID_LOCAL_SPECIAL_CHARS: &[char] = &[
    '!', '#', '$', '%', '&', '\'', '*', '+', '-', '/', '=', '?', '^', '_', '`', '{', '|', '}', '~', '.',
];

/// Validar sintaxis de correo electrónico con cumplimiento de RFC 5321 y RFC 5322
pub fn validate_email_syntax(email: &str) -> Result<(), ValidationError> {
    let email = email.trim();

    if email.is_empty() {
        return Err(ValidationError::InvalidEmail("El correo electrónico está vacío".to_string()));
    }

    // RFC 5321 Sección 4.5.3.1.3: La longitud total máxima de un path es 254 octetos
    if email.len() > 254 {
        return Err(ValidationError::InvalidEmail(
            "La longitud del correo excede el límite máximo de 254 caracteres (RFC 5321)".to_string(),
        ));
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() < 2 {
        return Err(ValidationError::InvalidEmail(
            "Formato de correo inválido: falta el carácter separador '@'".to_string(),
        ));
    }
    if parts.len() > 2 {
        return Err(ValidationError::InvalidEmail(
            "Formato de correo inválido: no puede contener múltiples caracteres '@'".to_string(),
        ));
    }

    let (local_part, domain) = (parts[0], parts[1]);

    // Validar parte local (local-part)
    validate_local_part(local_part)?;

    // Validar nombre de dominio (domain)
    validate_domain_part(domain)?;

    Ok(())
}

/// Validación detallada de la parte local (local_part)
fn validate_local_part(local: &str) -> Result<(), ValidationError> {
    if local.is_empty() {
        return Err(ValidationError::InvalidEmail(
            "La parte local (antes del '@') no puede estar vacía".to_string(),
        ));
    }

    // RFC 5321 Sección 4.5.3.1.1: local-part máximo 64 octetos
    if local.len() > 64 {
        return Err(ValidationError::InvalidEmail(
            "La parte local no puede exceder los 64 caracteres (RFC 5321)".to_string(),
        ));
    }

    // No puede comenzar ni terminar con un punto
    if local.starts_with('.') {
        return Err(ValidationError::InvalidEmail(
            "La parte local no puede comenzar con un punto (.)".to_string(),
        ));
    }
    if local.ends_with('.') {
        return Err(ValidationError::InvalidEmail(
            "La parte local no puede terminar con un punto (.)".to_string(),
        ));
    }

    // No puede contener puntos consecutivos
    if local.contains("..") {
        return Err(ValidationError::InvalidEmail(
            "La parte local no puede contener puntos consecutivos (..)".to_string(),
        ));
    }

    // Comprobación de caracteres válidos
    for c in local.chars() {
        if c.is_ascii_alphanumeric() || VALID_LOCAL_SPECIAL_CHARS.contains(&c) {
            continue;
        }
        if c.is_whitespace() {
            return Err(ValidationError::InvalidEmail(
                "La parte local no puede contener espacios".to_string(),
            ));
        }
        if c.is_ascii_control() {
            return Err(ValidationError::InvalidEmail(
                "La parte local contiene caracteres de control no permitidos".to_string(),
            ));
        }
        return Err(ValidationError::InvalidEmail(format!(
            "La parte local contiene un carácter no permitido: '{}'",
            c
        )));
    }

    Ok(())
}

/// Validación detallada del dominio según RFC 1035 y RFC 5321
fn validate_domain_part(domain: &str) -> Result<(), ValidationError> {
    if domain.is_empty() {
        return Err(ValidationError::InvalidEmail(
            "El dominio (después del '@') no puede estar vacío".to_string(),
        ));
    }

    // RFC 1035 / RFC 5321: Longitud máxima del dominio es 253 caracteres
    if domain.len() > 253 {
        return Err(ValidationError::InvalidEmail(
            "El dominio excede la longitud máxima permitida de 253 caracteres".to_string(),
        ));
    }

    if domain.starts_with('.') || domain.ends_with('.') {
        return Err(ValidationError::InvalidEmail(
            "El dominio no puede comenzar ni terminar con un punto (.)".to_string(),
        ));
    }

    if domain.contains("..") {
        return Err(ValidationError::InvalidEmail(
            "El dominio no puede contener puntos consecutivos (..)".to_string(),
        ));
    }

    let labels: Vec<&str> = domain.split('.').collect();

    // Debe contener al menos dos etiquetas (ejemplo: dominio.com)
    if labels.len() < 2 {
        return Err(ValidationError::InvalidEmail(
            "El dominio debe contener al menos dos niveles (ejemplo: dominio.com)".to_string(),
        ));
    }

    for (i, label) in labels.iter().enumerate() {
        if label.is_empty() {
            return Err(ValidationError::InvalidEmail(
                "El dominio contiene etiquetas vacías entre puntos".to_string(),
            ));
        }

        // Cada etiqueta individual no puede exceder 63 caracteres (RFC 1035)
        if label.len() > 63 {
            return Err(ValidationError::InvalidEmail(format!(
                "La etiqueta de dominio '{}' excede el límite de 63 caracteres",
                label
            )));
        }

        if label.starts_with('-') || label.ends_with('-') {
            return Err(ValidationError::InvalidEmail(format!(
                "La etiqueta de dominio '{}' no puede iniciar ni finalizar con guión (-)",
                label
            )));
        }

        for c in label.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' {
                if c.is_whitespace() {
                    return Err(ValidationError::InvalidEmail(
                        "El dominio no puede contener espacios".to_string(),
                    ));
                }
                return Err(ValidationError::InvalidEmail(format!(
                    "El dominio contiene caracteres no válidos: '{}'",
                    c
                )));
            }
        }

        // Validación específica del TLD (última etiqueta)
        if i == labels.len() - 1 {
            // El TLD debe tener al menos 2 caracteres
            if label.len() < 2 {
                return Err(ValidationError::InvalidEmail(
                    "El TLD (extensión del dominio) debe tener al menos 2 caracteres".to_string(),
                ));
            }
            // En dominios estándar, el TLD no debe ser puramente numérico
            if label.chars().all(|c| c.is_ascii_digit()) {
                return Err(ValidationError::InvalidEmail(
                    "El TLD no puede ser completamente numérico".to_string(),
                ));
            }
        }
    }

    Ok(())
}

/// Estructurar respuesta estándar de validación garantizando coherencia lógica
pub fn format_response(
    email: String,
    is_valid: bool,
    has_mx: bool,
    error_code: u32,
) -> EmailValidationResponse {
    let is_deliverable = is_valid && has_mx;
    let reason = if !is_valid {
        "Formato de correo inválido".to_string()
    } else if is_deliverable {
        "Correo válido y con registros MX encontrados".to_string()
    } else {
        "Correo con sintaxis válida pero sin registros MX o servidor de correo inalcanzable".to_string()
    };

    EmailValidationResponse {
        email,
        is_valid_syntax: is_valid,
        has_mx_records: has_mx,
        is_deliverable,
        error_code,
        reason,
    }
}

/// Estructurar respuesta con un motivo personalizado descriptivo
pub fn format_response_custom(
    email: String,
    is_valid: bool,
    has_mx: bool,
    error_code: u32,
    reason: String,
) -> EmailValidationResponse {
    let is_deliverable = is_valid && has_mx;
    EmailValidationResponse {
        email,
        is_valid_syntax: is_valid,
        has_mx_records: has_mx,
        is_deliverable,
        error_code,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emails() {
        assert!(validate_email_syntax("user@example.com").is_ok());
        assert!(validate_email_syntax("user.name+tag@sub.domain.org").is_ok());
        assert!(validate_email_syntax("user_name@domain.co.uk").is_ok());
    }

    #[test]
    fn test_invalid_syntax() {
        assert!(validate_email_syntax("").is_err());
        assert!(validate_email_syntax("plainaddress").is_err());
        assert!(validate_email_syntax("@missinglocal.com").is_err());
        assert!(validate_email_syntax("user@.com").is_err());
        assert!(validate_email_syntax("user@com.").is_err());
        assert!(validate_email_syntax("user@domain..com").is_err());
        assert!(validate_email_syntax(".user@domain.com").is_err());
        assert!(validate_email_syntax("user.@domain.com").is_err());
        assert!(validate_email_syntax("user..name@domain.com").is_err());
        assert!(validate_email_syntax("user@domain.c").is_err());
    }

    #[test]
    fn test_logical_coherence_in_format_response() {
        let resp = format_response("test@bad".to_string(), false, true, shared::error_codes::ERR_INVALID_SYNTAX);
        assert!(!resp.is_valid_syntax);
        assert!(!resp.is_deliverable);
        assert_eq!(resp.error_code, 2);
        assert_eq!(resp.reason, "Formato de correo inválido");
    }
}