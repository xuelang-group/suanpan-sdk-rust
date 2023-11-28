use base64::Engine as _;
pub fn decode_b64(s: &str) -> crate::types::SuanpanResult<String> {
    if let Ok(buf) = base64::engine::general_purpose::STANDARD.decode(s) {
        Ok(String::from_utf8(buf).unwrap())
    } else {
        Err(crate::types::SuanpanError::from((
            "DecodeB64Error",
            format!("decode b64 error: {}", s),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_decode_b64() {
        let testb64 = r#"YXNkZ2FzZGdhc2RnYXNkZ2FzZGdhc2RnYXNkZ2RzZ3Nk"#;
        let r = decode_b64(testb64);
        assert_eq!(r.is_ok(), true);
    }
}
