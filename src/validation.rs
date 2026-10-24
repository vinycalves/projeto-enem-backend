use crate::error::AppError;

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn validate_email(email: &str) -> Result<(), AppError> {
    if email.len() < 5 || email.len() > 254 {
        return Err(AppError::BadRequest(
            "Informe um e-mail válido.".to_string(),
        ));
    }

    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();

    if local.is_empty()
        || domain.is_empty()
        || parts.next().is_some()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || email.contains(' ')
    {
        return Err(AppError::BadRequest(
            "Informe um e-mail válido.".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::BadRequest(
            "A senha precisa ter pelo menos 8 caracteres.".to_string(),
        ));
    }

    if password.len() > 128 {
        return Err(AppError::BadRequest(
            "A senha pode ter no máximo 128 caracteres.".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_option(option: &str) -> Result<String, AppError> {
    let normalized = option.trim().to_uppercase();
    if matches!(normalized.as_str(), "A" | "B" | "C" | "D" | "E") {
        Ok(normalized)
    } else {
        Err(AppError::BadRequest(
            "A alternativa deve ser A, B, C, D ou E.".to_string(),
        ))
    }
}

pub fn validate_comment_body(body: &str) -> Result<String, AppError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(
            "Escreva um comentário antes de enviar.".to_string(),
        ));
    }
    if trimmed.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "O comentário pode ter no máximo 1000 caracteres.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

pub fn display_name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("Estudante");
    let cleaned = local.replace(['.', '_', '-'], " ");
    let name = cleaned
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    if name.is_empty() {
        "Estudante".to_string()
    } else {
        name.chars().take(40).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_normal_email() {
        assert!(validate_email("aluno@exemplo.com").is_ok());
    }

    #[test]
    fn rejects_invalid_email() {
        assert!(validate_email("aluno").is_err());
        assert!(validate_email("aluno@").is_err());
        assert!(validate_email("@exemplo.com").is_err());
        assert!(validate_email("aluno@exemplo").is_err());
    }

    #[test]
    fn validates_password_length() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("1234567").is_err());
    }

    #[test]
    fn normalizes_option() {
        assert_eq!(validate_option("b").unwrap(), "B");
        assert!(validate_option("F").is_err());
    }

    #[test]
    fn creates_display_name() {
        assert_eq!(
            display_name_from_email("vinycius.florencio@exemplo.com"),
            "Vinycius Florencio"
        );
    }
}
