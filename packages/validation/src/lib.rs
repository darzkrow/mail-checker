//! email-validator-validation - Validación de sintaxis de email
use shared::{EmailValidationResponse, ValidationError};

/// Validar sintaxis de email (RFC 5322 simplificada)
pub fn validate_email_syntax(email: &str) -> Result<(), ValidationError> {
    let email = email.trim();

    if email.is_empty() {
        return Err(ValidationError::InvalidEmail("Correo electrónico vacío".to_string()));
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(ValidationError::InvalidEmail(
            "Formato de correo inválido: debe contener exactamente un @".to_string(),
        ));
    }

    let (local_part, domain) = (parts[0], parts[1]);

    if local_part.is_empty() {
        return Err(ValidationError::InvalidEmail(
            "El nombre local (parte antes de @) está vacío".to_string(),
        ));
    }

    if domain.is_empty() {
        return Err(ValidationError::InvalidEmail("El dominio está vacío".to_string()));
    }

    if !domain.contains('.') {
        return Err(ValidationError::InvalidEmail(
            "El dominio debe contener al menos un punto (.)".to_string(),
        ));
    }

    if local_part.contains(' ') {
        return Err(ValidationError::InvalidEmail(
            "El nombre local no puede contener espacios".to_string(),
        ));
    }

    if domain.contains(' ') {
        return Err(ValidationError::InvalidEmail(
            "El dominio no puede contener espacios".to_string(),
        ));
    }

    Ok(())
}

/// Estructurar respuesta después de validación exitosa
pub fn format_response(email: String, is_valid: bool, has_mx: bool) -> EmailValidationResponse {
    let is_deliverable = is_valid && has_mx;
    let reason = if has_mx {
        "Correo válido y con registros MX encontrados".to_string()
    } else if is_valid {
        "Correo con sintaxis válida pero sin registros MX".to_string()
    } else {
        "Formato de correo inválido".to_string()
    };

    EmailValidationResponse {
        email,
        is_valid_syntax: is_valid,
        has_mx_records: has_mx,
        is_deliverable,
        reason,
    }
}