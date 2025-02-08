/// Login/new registration views.
use crate::components::ui::*;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use std::collections::HashMap;

#[cfg(feature = "ssr")]
use crate::app_state::*;
#[cfg(feature = "ssr")]
use crate::key;

#[cfg(feature = "ssr")]
use fred::prelude::{HashesInterface, KeysInterface, TransactionInterface};

const LOGIN_SESSION_EXPIRATION_MIN: i64 = 20;

#[cfg(feature = "ssr")]
const CHALLENGE_LENGTH: usize = 32;
const RESPONSE_LENGTH: usize = 8;

/// Email authentication first stage, where a challenge is generated and
/// returned, while the correct response is sent via email.
///
/// See https://en.wikipedia.org/wiki/Challenge%E2%80%93response_authentication
#[server]
async fn get_email_login_challenge(email: String) -> Result<String, ServerFnError> {
    use crate::mail;

    use lettre::AsyncTransport;
    use rand::{
        distributions::{Alphanumeric, DistString},
        thread_rng,
    };

    let address = match email.parse::<lettre::address::Address>() {
        Ok(email) => email,
        Err(_) => return Err(ServerFnError::new("Bad email")),
    };

    let app_state = use_app_state()?;
    let challenge = Alphanumeric.sample_string(&mut thread_rng(), CHALLENGE_LENGTH);
    let response = Alphanumeric.sample_string(&mut thread_rng(), RESPONSE_LENGTH);

    let message = mail::login_code(address, &response, LOGIN_SESSION_EXPIRATION_MIN)
        .or_else(|err| Err(ServerFnError::new(format!("Couldn't send mail: {err}"))))?;
    app_state
        .mailer
        .send(message)
        .await
        .or_else(|err| Err(ServerFnError::new(format!("Couldn't send mail: {err}"))))?;

    let tx = app_state.valkey_pool.multi();
    let key = key::email_auth_code(&challenge);
    let _: () = tx
        .hset(
            &key,
            HashMap::from([("email", email), ("response", response)]),
        )
        .await?;
    let _: () = tx
        .expire(&key, LOGIN_SESSION_EXPIRATION_MIN * 60, None)
        .await?;
    let _: () = tx.exec(false).await?;
    Ok(challenge)
}

/// Email authentication second stage, where the challenge is answered.
///
/// Note that, for security reasons, we can't tell the user which exactly of
/// (email, challenge, response) was wrong.
#[server]
async fn answer_email_login_challenge(
    email: String,
    challenge: String,
    response: String,
) -> Result<bool, ServerFnError> {
    use uuid::Uuid;

    if email.len() <= 0
        || challenge.len() != CHALLENGE_LENGTH
        || response.len() != RESPONSE_LENGTH
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
        None => return Ok(false), // Different email = wrong login.
    };
    let correct_response = match correct_data.get("response") {
        Some(value) => value,
        None => return Ok(false), // Wrong response = wrong login.
    };
    if email == *correct_email && response == *correct_response {
        tokio::spawn(async move {
            if let Err(err) = app_state.valkey_pool.del::<(), _>(&key).await {
                leptos::logging::warn!("Error deleting key {key} ignored: {err}");
            }
        });

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
            Some((account_id, ask_for_profile_on_login, username, display_name)) => {
                leptos::logging::log!("You do have an account");
            }
            None => {
                leptos::logging::log!("You don't have an account");
            }
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Main login page.
#[component]
pub fn Login() -> impl IntoView {
    let get_email_login_challenge = ServerAction::<GetEmailLoginChallenge>::new();
    let answer_email_login_challenge = ServerAction::<AnswerEmailLoginChallenge>::new();

    let email = RwSignal::new("".to_string());
    let challenge = RwSignal::new("".to_string());
    // The last email address that was submitted (not the one currently entered
    // into the form).
    let last_email = RwSignal::new("".to_string());

    let _save_last_email = Effect::new(move || {
        // input is cleared as soon as the server action resolves, so we must
        // save this so the UI doesn't update then.
        if let Some(server_last_email) = get_email_login_challenge.input().get() {
            last_email.set(server_last_email.email);
        }
    });

    let code_input_elem = NodeRef::<leptos::html::Input>::new();

    // Handler after receiving the challenge from the server.
    let _receive_challenge = Effect::new(move || {
        if let Some(Ok(server_challenge)) = get_email_login_challenge.value().get() {
            challenge.set(server_challenge);
            if let Some(node) = code_input_elem.get() {
                if let Err(err) = node.focus() {
                    leptos::logging::warn!("Error focusing code input: {err:?}");
                }
            } else {
                leptos::logging::warn!("Wanted to focus code input, but it wasn't mounted");
            }
        }
    });

    view! {
        <h1 class="text-xl font-bold">"Login/register"</h1>

        <fieldset class="pt-1 pb-2 my-2 border-2 border-slate-500">
            <legend class="font-bold text-l">Email challenge</legend>

            <p>Receive and input a login code sent to the given email address.</p>

            <ActionForm action=get_email_login_challenge>
                <div class="flex gap-2">
                    <label for="email">Email:</label>
                    <input
                        type="email"
                        name="email"
                        placeholder="email"
                        class="px-1 h-full bg-gray-200 border border-gray-500 invalid:border-red-500"
                        required
                        bind:value=email
                    />
                    <input
                        type="submit"
                        value="Email me"
                        class="px-2 h-full bg-green-200 hover:bg-green-300"
                    />
                    <Show when=move || { *get_email_login_challenge.pending().read() }>
                        <span class="self-center">
                            <Spinner />
                        </span>
                    </Show>
                </div>
            </ActionForm>

            <Show
                when=move || {
                    get_email_login_challenge.value().with(|val| matches!(val, Some(Ok(_))))
                }
                fallback=move || {
                    view! {
                        {move || {
                            match get_email_login_challenge.value().get() {
                                Some(Err(err)) => {
                                    view! { <ShowServerFnError error=err /> }.into_any()
                                }
                                _ => view! {}.into_any(),
                            }
                        }}
                    }
                }
            >
                <p>
                    "An email has been sent to " {last_email}
                    " with a login code; please enter it here within "
                    {LOGIN_SESSION_EXPIRATION_MIN} " minutes".
                </p>

                <ActionForm action=answer_email_login_challenge>
                    <div class="flex gap-2">
                        <input type="hidden" name="email" placeholder="email" bind:value=email />
                        <input
                            type="hidden"
                            name="challenge"
                            placeholder="challenge"
                            bind:value=challenge
                        />
                        <label for="response">Login code:</label>
                        <input
                            type="text"
                            name="response"
                            placeholder="response"
                            class="px-1 h-full bg-gray-200 border border-gray-500 invalid:border-red-500"
                            minlength=RESPONSE_LENGTH
                            maxlength=RESPONSE_LENGTH
                            pattern="^[A-Za-z0-9]*$"
                            title=format!(
                                "exactly {RESPONSE_LENGTH} uppercase, lowercase, or numeric characters",
                            )
                            required
                            node_ref=code_input_elem
                        />
                        <input
                            type="submit"
                            value="Submit code"
                            class="px-2 h-full bg-green-200 hover:bg-green-300"
                        />
                        <Show when=move || { answer_email_login_challenge.pending().get() }>
                            <span class="self-center">
                                <Spinner />
                            </span>
                        </Show>
                    </div>
                </ActionForm>

                <Show
                    when=move || {
                        answer_email_login_challenge.value().with(|val| matches!(val, Some(Ok(_))))
                    }
                    fallback=move || {
                        view! {
                            {move || {
                                if let Some(Err(err)) = answer_email_login_challenge.value().get() {
                                    view! { <ShowServerFnError error=err /> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }
                            }}
                        }
                    }
                >
                    {move || {
                        if let Some(Ok(accepted)) = answer_email_login_challenge.value().get() {
                            if accepted {
                                "Login code accepted."
                            } else {
                                "Login code rejected. Try again."
                            }
                        } else {
                            ""
                        }
                    }}
                </Show>
            </Show>

        </fieldset>
    }
}
