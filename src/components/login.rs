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
use crate::mail;

#[cfg(feature = "ssr")]
use fred::prelude::{HashesInterface, KeysInterface, TransactionInterface};

#[cfg(feature = "ssr")]
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

#[cfg(feature = "ssr")]
use lettre::AsyncTransport;

#[allow(dead_code)] // Used in a server function, but rustc doesn't count it.
const LOGIN_SESSION_EXPIRATION_SEC: i64 = 20 * 60; // 20 minutes

/// Email authentication first stage, where a challenge is generated and
/// returned, while the correct response is sent via email.
///
/// See https://en.wikipedia.org/wiki/Challenge%E2%80%93response_authentication
#[server]
pub async fn get_email_login_challenge(email: String) -> Result<String, ServerFnError> {
    let app_state = use_app_state()?;
    let challenge = Alphanumeric.sample_string(&mut thread_rng(), 32);
    let response = Alphanumeric.sample_string(&mut thread_rng(), 8);

    let message = mail::login_code(&email, &response)
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
    let _: () = tx.expire(&key, LOGIN_SESSION_EXPIRATION_SEC, None).await?;
    let _: () = tx.exec(false).await?;
    Ok(challenge)
}

/// Email authentication second stage, where the challenge is answered.
///
/// Note that, for security reasons, we can't tell the user which exactly of
/// (email, challenge, response) was wrong.
#[server]
pub async fn answer_email_login_challenge(
    email: String,
    challenge: String,
    response: String,
) -> Result<bool, ServerFnError> {
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
                            if let Some(Err(err)) = get_email_login_challenge.value().get() {
                                format!("Error: {err}")
                            } else {
                                String::new()
                            }
                        }}
                    }
                }
            >
                <p>
                    An email has been sent to {last_email}
                    with a login code; please enter it here within 20 minutes.
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
                                    format!("Error: {err}")
                                } else {
                                    String::new()
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
