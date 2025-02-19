pub fn email_auth_code(secret: &str) -> String {
    format!("emauthsec:{secret}")
}

pub fn session(session_id: &str) -> String {
    format!("sess:{session_id}")
}

pub fn new_registration(secret: &str) -> String {
    format!("regnew:{secret}")
}
