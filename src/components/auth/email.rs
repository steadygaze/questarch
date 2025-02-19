use crate::components::ui::*;

use codee::string::FromToStringCodec;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::{MatchNestedRoutes, path};
use leptos_use::use_cookie;

#[cfg(feature = "ssr")]
use crate::key;
#[cfg(feature = "ssr")]
use crate::mail;
#[cfg(feature = "ssr")]
mod ssr {
    pub use crate::app_state::*;
    pub use crate::cookie::*;
    pub use actix_web::HttpRequest;
    pub use actix_web::cookie;
    pub use actix_web::cookie::Cookie;
    pub use fred::prelude::{HashesInterface, KeysInterface, TransactionInterface};
    pub use leptos_actix::extract;
    pub use lettre::AsyncTransport;
    pub use rand::{
        distributions::{Alphanumeric, DistString},
        thread_rng,
    };
    pub use std::collections::HashMap;
    pub use uuid::Uuid;

    pub const CHALLENGE_REGCODE_LEN: usize = 16;
    pub const REGISTRATION_CODE_EXPIRATION_MIN: i64 = 60 * 2;
}

const LOGIN_CODE_EXPIRATION_MIN: i64 = 20;
const RESPONSE_LEN: usize = 8;

/// Route definitions for email auth stages.
#[component(transparent)]
pub fn Routes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("email") view=EmailContainer>
            <Route path=path!("") view=Start />
            <Route path=path!("challenge") view=Challenge />
        </ParentRoute>
    }
    .into_inner()
}

/// Email authentication first stage, where a challenge is generated while the correct response is
/// sent via email.
///
/// See https://en.wikipedia.org/wiki/Challenge%E2%80%93response_authentication
#[server]
async fn get_email_login_challenge(email: String) -> Result<(), ServerFnError> {
    use self::ssr::*;

    leptos::logging::log!("get_email_login_challenge exercised");

    let address = match email.parse::<lettre::address::Address>() {
        Ok(email) => email,
        Err(_) => return Err(ServerFnError::new("Bad email")),
    };

    let app_state = use_app_state()?;
    let response = Alphanumeric.sample_string(&mut thread_rng(), RESPONSE_LEN);

    let message = mail::login_code(address, &response, LOGIN_CODE_EXPIRATION_MIN)
        .or_else(|err| Err(ServerFnError::new(format!("Couldn't send mail: {err}"))))?;
    app_state
        .mailer
        .send(message)
        .await
        .or_else(|err| Err(ServerFnError::new(format!("Couldn't send mail: {err}"))))?;

    let challenge = {
        let mut challenge = String::new();
        let mut key = String::new();
        for i in 0..=10 {
            if i >= 10 {
                // This may happen in a very severe DDoS if there are no rate limiting
                // countermeasures at all. Otherwise, it should never happen.
                return Err(ServerFnError::new(
                    "Couldn't generate a unique registration code. There may be a serious problem with entropy sources or keyspace saturation.",
                ));
            }

            challenge = Alphanumeric.sample_string(&mut thread_rng(), CHALLENGE_REGCODE_LEN);
            key = key::email_auth_code(&challenge);

            if app_state.valkey_pool.exists::<i32, _>(&key).await? <= 0 {
                break;
            }
        }

        let tx = app_state.valkey_pool.multi();
        let _: () = tx
            .hset(&key, [("email", &email), ("response", &response)])
            .await?;
        let _: () = tx
            .expire(&key, LOGIN_CODE_EXPIRATION_MIN * 60, None)
            .await?;
        let _: () = tx.exec(false).await?;
        challenge
    };

    let response_options = use_response_options()?;

    let challenge_cookie = Cookie::build("lgchal", challenge)
        .path("/")
        .max_age(cookie::time::Duration::minutes(LOGIN_CODE_EXPIRATION_MIN))
        .same_site(cookie::SameSite::Lax)
        .finish();

    let email_cookie = Cookie::build("lgmail", email)
        .path("/") // Must be / for SSR; server functions will be under /api.
        .max_age(cookie::time::Duration::minutes(LOGIN_CODE_EXPIRATION_MIN))
        .same_site(cookie::SameSite::Lax)
        .finish();

    set_cookie(&response_options, &challenge_cookie)?;
    set_cookie(&response_options, &email_cookie)?;

    leptos_actix::redirect("/auth/email/challenge");

    Ok(())
}

/// Check the answer to a user's login challenge.
///
/// If correct, also redirect to the home or registration page, depending on whether the user has
/// an account or not.
///
/// Note that, for security reasons, we can't tell the user which exactly of
/// (email, challenge, response) was wrong.
#[server]
async fn answer_email_login_challenge(response: String) -> Result<bool, ServerFnError> {
    use self::ssr::*;

    let request: HttpRequest = extract().await?;
    let email = get_cookie(&request, "lgmail");
    let challenge = get_cookie(&request, "lgchal");

    if email.len() <= 0
        || challenge.len() != CHALLENGE_REGCODE_LEN
        || response.len() != RESPONSE_LEN
        || !challenge.chars().all(char::is_alphanumeric)
        || !response.chars().all(char::is_alphanumeric)
    {
        leptos::logging::debug_warn!("Rejecting invalid login challenge inputs");
        // Note that the actual form should never send these inputs.
        return Ok(false);
    }

    let app_state = use_app_state()?;
    let key = key::email_auth_code(&challenge);
    let correct_data: HashMap<String, String> = match app_state
        .valkey_pool
        .hgetall::<Option<HashMap<String, String>>, _>(&key)
        .await?
    {
        Some(value) => value,
        None => return Ok(false), // No matching challenge = wrong login.
    };

    let correct_email = match correct_data.get("email") {
        Some(value) => value,
        None => return Ok(false), // No email = wrong login.
    };
    let correct_response = match correct_data.get("response") {
        Some(value) => value,
        None => return Ok(false), // No response = wrong login.
    };
    if email != *correct_email || response != *correct_response {
        return Ok(false); // Wrong email or response = wrong login.
    }

    {
        let valkey_pool = app_state.valkey_pool.clone();
        // Response accepted; clean it up as it's a one-time code.
        tokio::spawn(async move {
            if let Err(err) = valkey_pool.del::<(), _>(&key).await {
                leptos::logging::warn!("Error deleting key {key} ignored: {err}");
            }
        });
    }

    let response_options = use_response_options()?;

    remove_cookie(&response_options, "lgchal")?;
    remove_cookie(&response_options, "lgmail")?;

    match sqlx::query_as::<_, (Uuid, bool, Option<String>, Option<String>)>(
        r#"
        select
          account.id,
          ask_for_profile_on_login,
          profile.username,
          profile.display_name
        from
          account
          left join profile on account.default_profile = profile.id
        where
          email = $1
          or $1 = any(secondary_email)
        limit 1
        "#,
    )
    .bind(&email)
    .fetch_optional(&app_state.db_pool)
    .await
    .or_else(|err| {
        Err(ServerFnError::new(format!(
            "Couldn't get account from DB: {err}"
        )))
    })? {
        Some((account_id, _ask_for_profile_on_login, username, display_name)) => {
            app_state
                .create_session(&response_options, account_id, username, display_name)
                .await?;
            // TODO - Redirect to profile picker if applicable.
            leptos_actix::redirect("/");
            Ok(true)
        }
        None => {
            let registration_code = {
                let mut registration_code = String::new();
                for i in 0..=10 {
                    if i >= 10 {
                        // This may happen in a very severe DDoS if there are no rate limiting
                        // countermeasures at all. Otherwise, it should never happen.
                        return Err(ServerFnError::new(
                            "Couldn't generate a unique registration code. There may be a serious problem with entropy sources or keyspace saturation.",
                        ));
                    }
                    registration_code =
                        Alphanumeric.sample_string(&mut thread_rng(), CHALLENGE_REGCODE_LEN);
                    if app_state
                        .valkey_pool
                        .set(
                            key::new_registration(&registration_code),
                            &email,
                            Some(fred::types::Expiration::EX(
                                REGISTRATION_CODE_EXPIRATION_MIN * 60,
                            )),
                            Some(fred::prelude::SetOptions::NX),
                            false,
                        )
                        .await?
                    {
                        break;
                    }
                }
                registration_code
            };

            // Registration code is an HTTP only cookie, with similar security to a session token.
            let registration_code_cookie = Cookie::build("regcode", registration_code)
                .max_age(cookie::time::Duration::minutes(
                    REGISTRATION_CODE_EXPIRATION_MIN,
                ))
                .same_site(cookie::SameSite::Lax)
                .path("/")
                .http_only(true)
                // .secure(true) // No dev https setup.
                .finish();

            // Can't use the same login email cookie because it has a different max_age.
            let registration_email_cookie = Cookie::build("regmail", email)
                .max_age(cookie::time::Duration::minutes(
                    REGISTRATION_CODE_EXPIRATION_MIN,
                ))
                .same_site(cookie::SameSite::Lax)
                .path("/")
                .finish();

            set_cookie(&response_options, &registration_code_cookie)?;
            set_cookie(&response_options, &registration_email_cookie)?;

            leptos_actix::redirect("/auth/register");
            Ok(true)
        }
    }
}

/// Common email challenge info.
#[component]
pub fn EmailContainer() -> impl IntoView {
    view! {
        <fieldset class="px-2 pt-1 pb-2 mb-2 border-2 border-slate-500">
            <legend class="mx-2 text-2xl font-bold">Email challenge</legend>
            <p>Receive and input a login code sent to the given email address.</p>
            <Outlet />
        </fieldset>
    }
}

#[component]
pub fn Start() -> impl IntoView {
    let get_email_login_challenge = ServerAction::<GetEmailLoginChallenge>::new();
    view! {
        <ActionForm action=get_email_login_challenge>
            <div class="flex gap-2">
                <label for="email">Email:</label>
                <input
                    type="email"
                    name="email"
                    placeholder="email"
                    class="px-1 h-full bg-gray-200 border border-gray-500 invalid:border-red-500"
                    required
                />
                <input
                    type="submit"
                    value="Email me"
                    class="px-2 h-full bg-green-200 hover:bg-green-300"
                />
            </div>
        </ActionForm>
    }
}

#[component]
pub fn Challenge() -> impl IntoView {
    let (email, _) = use_cookie::<String, FromToStringCodec>("lgmail");

    let answer_email_login_challenge = ServerAction::<AnswerEmailLoginChallenge>::new();

    view! {
        <Show when=move || email.read().is_none()>
            <Redirect path=".." />
        </Show>

        <ActionForm action=answer_email_login_challenge>
            <div class="flex gap-2">
                <label for="email">Email:</label>
                <input
                    type="email"
                    name="email"
                    placeholder="email"
                    class="px-1 h-full bg-gray-200 border border-gray-500 invalid:border-red-500"
                    required
                    // We have the email as a cookie already; we don't have to resend it.
                    disabled
                    value=email
                />
                <input
                    type="submit"
                    value="Email me"
                    class="px-2 h-full bg-green-200 hover:bg-green-300"
                    disabled
                />
            </div>

            <p>
                "An email has been sent to " {move || email()}
                " with a login code; please enter it here within " {LOGIN_CODE_EXPIRATION_MIN}
                " minutes".
            </p>

            <div class="flex gap-2">
                <label for="response">Login code:</label>
                <input
                    type="text"
                    name="response"
                    placeholder="response"
                    class="px-1 h-full bg-gray-200 border border-gray-500 invalid:border-red-500"
                    minlength=RESPONSE_LEN
                    maxlength=RESPONSE_LEN
                    pattern="^[A-Za-z0-9]*$"
                    title=format!(
                        "exactly {RESPONSE_LEN} uppercase, lowercase, or numeric characters",
                    )
                    required
                    autofocus
                    autocomplete="off"
                    value=""
                />
                <input
                    type="submit"
                    value="Submit code"
                    class="px-2 h-full bg-green-200 hover:bg-green-300"
                />
                <Show
                    when=move || { !answer_email_login_challenge.pending().get() }
                    fallback=move || view! { <Spinner /> }
                >
                    {move || {
                        if let Some(value) = answer_email_login_challenge.value().get() {
                            if let Err(err) = value {
                                view! { <ShowServerFnError error=err /> }.into_any()
                            } else if let Ok(true) = value {
                                view! {
                                    "Login code accepted. You will be automatically redirected shortly."
                                }
                                    .into_any()
                            } else {
                                view! { "Login code rejected. Try again." }.into_any()
                            }
                        } else {
                            view! { "" }.into_any()
                        }
                    }}
                </Show>
            </div>
        </ActionForm>
    }
    .into_any()
}
