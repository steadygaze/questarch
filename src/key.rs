pub fn email_auth_code(secret: &str) -> String {
    format!("email_auth_secret:{secret}")
}
