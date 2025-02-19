use crate::ssr::cookie::set_cookie;
use crate::ssr::key;

use actix_web::HttpRequest;
use actix_web::cookie;
use actix_web::cookie::Cookie;
use fred::interfaces::HashesInterface;
use fred::interfaces::KeysInterface;
use fred::interfaces::TransactionInterface;
use leptos::prelude::*;
use leptos_actix::ResponseOptions;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};
use uuid::Uuid;

// See https://owasp.org/www-community/vulnerabilities/Insufficient_Session-ID_Length for
// considerations for secret lengths.
const SESSION_ID_LEN: usize = 16;
const SESSION_ID_COOKIE: &str = "sess";
const SESSION_TTL_DAYS: i64 = 180; // 30 days
const SESSION_TTL_SEC: i64 = SESSION_TTL_DAYS * 24 * 60 * 60; // 180 days

/// Easily cloneable prototype.
#[allow(dead_code)] // For prototyping
#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::postgres::PgPool,
    pub valkey_pool: fred::clients::Pool,
    pub mailer: AsyncSmtpTransport<Tokio1Executor>,
}

/// Data associated with a session.
pub struct SessionInfo {
    pub account_id: Uuid,
    pub session_id: String,
    pub username: String,
    pub display_name: String,
}

impl AppState {
    /// Helper to get a user's session details.
    pub async fn get_session(
        &self,
        request: HttpRequest,
    ) -> Option<Result<SessionInfo, ServerFnError>> {
        if let Some(session_cookie) = request.cookie(SESSION_ID_COOKIE) {
            Some(self.get_session_for(session_cookie.value()).await)
        } else {
            None
        }
    }

    /// Helper to create a session for a new given user.
    pub async fn create_session(
        &self,
        response_options: &ResponseOptions,
        account_id: Uuid,
        username: Option<String>,
        display_name: Option<String>,
    ) -> Result<(), ServerFnError> {
        let session_id = Alphanumeric.sample_string(&mut thread_rng(), SESSION_ID_LEN);

        let transaction = self.valkey_pool.multi();
        if username.is_some() {
            let _ = transaction
                .hset::<i64, _, _>(
                    key::session(&session_id),
                    [
                        ("acctid", account_id.simple().to_string()),
                        ("uname", username.unwrap_or_default()),
                        ("dname", display_name.unwrap_or_default()),
                    ],
                )
                .await;
        } else {
            let _ = transaction
                .hset::<i64, _, _>(
                    key::session(&session_id),
                    ("acctid", account_id.simple().to_string()),
                )
                .await;
        }
        let _ = transaction
            .expire::<i64, _>(key::session(&session_id), SESSION_TTL_SEC, None)
            .await;

        transaction.exec::<(i64, i64)>(true).await.or_else(|err| {
            Err(ServerFnError::new(format!(
                "Failed to create session: {err}"
            )))
        })?;

        let session_cookie = Cookie::build("sess", session_id)
            .max_age(cookie::time::Duration::days(SESSION_TTL_DAYS))
            .same_site(cookie::SameSite::Lax)
            .path("/")
            .http_only(true)
            // .secure(true) // no dev HTTPS setup
            .finish();
        set_cookie(response_options, &session_cookie)?;

        Ok(())
    }

    /// Helper function for clearing the server's session record. This has to be
    /// done if we notice it's corrupted in some way.
    fn background_clear_session(&self, session_id: &str) {
        let session_id = session_id.to_string();
        let valkey_pool = self.valkey_pool.clone();
        tokio::spawn(async move {
            if let Err(err) = valkey_pool
                .del::<String, _>(key::session(&session_id))
                .await
            {
                log::warn!("Ignored error clearing invalid session entry: {err}");
            }
        });
    }

    /// Helper to check if a session ID is in the right format.
    fn valid_session_id(&self, session_id: &str) -> bool {
        session_id.len() == SESSION_ID_LEN && session_id.chars().all(char::is_alphanumeric)
    }

    /// Private helper for getting a specific session.
    async fn get_session_for(&self, session_id: &str) -> Result<SessionInfo, ServerFnError> {
        if !self.valid_session_id(session_id) {
            return Err(ServerFnError::new(
                "Your session was corrupted. Try logging in again.",
            ));
        }

        let [account_id, username, display_name] = self
            .valkey_pool
            .hmget::<[String; 3], _, _>(key::session(session_id), ("acctid", "uname", "dname"))
            .await
            .or_else(|err| {
                Err(ServerFnError::new(format!(
                    "Failed to commit account creation: {err}"
                )))
            })?;

        if account_id.is_empty() {
            if !username.is_empty() || !display_name.is_empty() {
                // A session hash with no account_id is invalid. Delete it.
                self.background_clear_session(session_id);
            }
            return Err(ServerFnError::new(
                "Your session expired or was corrupted. Try logging in again.",
            ));
        }

        let account_id = match Uuid::try_parse(&account_id) {
            Ok(account_id) => account_id,
            Err(err) => {
                log::error!("Unparseable account_id \"{}\": {}", account_id, err);
                self.background_clear_session(session_id);
                return Err(ServerFnError::new(
                    "Your session was corrupted. Try logging in again.",
                ));
            }
        };

        Ok(SessionInfo {
            account_id,
            session_id: session_id.to_string(),
            username,
            display_name,
        })
    }
}

/// Wrapper to get AppState that's easily usable with the ? operator, for use in
/// server functions.
pub fn use_app_state() -> Result<AppState, ServerFnError> {
    match use_context::<AppState>() {
        Some(app_state) => Ok(app_state),
        None => Err(ServerFnError::new("Couldn't get AppState from context")),
    }
}
