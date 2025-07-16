#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    // 长度不够
    #[error("key length must be at least 64 characters")]
    LengthTooShort,
    // 包含非法字符
    #[error("key contains illegal characters, must be alphanumeric, '+', '/', or '='")]
    IllegalCharacters,
}

pub fn check_key(key: &str) -> Result<(), KeyError> {
    if key.len() < 64 {
        return Err(KeyError::LengthTooShort);
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return Err(KeyError::IllegalCharacters);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_key() {
        assert!(
            check_key("ls/Jh1rYnCIiMCiYlIKxf0XXbMHNMAQW4/YkfVFQgyW7VX3jx68m6CFjt1NtlEUh").is_ok()
        );
        assert!(matches!(check_key("short"), Err(KeyError::LengthTooShort)));
        assert!(matches!(
            check_key(
                "invalid@key/invalid@key/invalid@key/invalid@key/invalid@key/invalid@key/invalid@key"
            ),
            Err(KeyError::IllegalCharacters)
        ));
    }
}
