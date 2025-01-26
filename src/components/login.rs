#![allow(unused_variables)]
#![allow(dead_code)]

#[cfg(feature = "ssr")]
use std::collections::HashMap;

#[cfg(feature = "ssr")]
use crate::app_state::*;
#[cfg(feature = "ssr")]
use crate::key;

#[cfg(feature = "ssr")]
use fred::prelude::{HashesInterface, KeysInterface, TransactionInterface};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

const LOGIN_SESSION_EXPIRATION_SEC: i64 = 6 * 60 * 60;

#[server]
pub async fn get_email_login_challenge(email: String) -> Result<String, ServerFnError> {
    let app_state = use_app_state()?;
    let challenge = Alphanumeric.sample_string(&mut thread_rng(), 32);
    let response = Alphanumeric.sample_string(&mut thread_rng(), 8);
    leptos::logging::log!("Challenge: {challenge}, response: {response}");

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

#[server]
pub async fn answer_email_login_challenge(
    email: String,
    challenge: String,
    response: String,
) -> Result<bool, ServerFnError> {
    let say_login_incorrect = || ServerFnError::new("Incorrect login information submitted");

    let app_state = use_app_state()?;
    let key = key::email_auth_code(&challenge);
    let correct_data: HashMap<String, String> = app_state
        .valkey_pool
        .hgetall::<Option<HashMap<String, String>>, _>(&key)
        .await?
        .ok_or_else(say_login_incorrect)?;

    let correct_email = correct_data.get("email").ok_or_else(say_login_incorrect)?;
    let correct_response = correct_data
        .get("response")
        .ok_or_else(say_login_incorrect)?;
    if email == *correct_email && response == *correct_response {
        tokio::spawn(async move {
            if let Err(err) = app_state.valkey_pool.del::<(), _>(&key).await {
                leptos::logging::warn!("Error deleting key {key} ignored");
            }
        });
        Ok(true)
    } else {
        Ok(false)
    }
}

#[component]
pub fn Login() -> impl IntoView {
    let get_email_login_challenge = ServerAction::<GetEmailLoginChallenge>::new();
    let answer_email_login_challenge = ServerAction::<AnswerEmailLoginChallenge>::new();
    // holds the latest *returned* value from the server
    let value = get_email_login_challenge.value();
    // check if the server has returned an error
    let has_error = move || value.with(|val| matches!(val, Some(Err(_))));

    view! {
        <h1 class="text-xl font-bold">"Login/register"</h1>

        <ActionForm action=get_email_login_challenge>
            <label for="email">Email challenge:</label>
            <input
                type="email"
                name="email"
                placeholder="email"
                class="p-1 bg-gray-200 border border-gray-500 invalid:border-red-500"
            />
            <input
                type="submit"
                value="Email me"
                class="py-0.5 px-2 bg-green-200 hover:bg-green-300"
            />
        </ActionForm>

        <p>{move || if get_email_login_challenge.pending().get() { "Loading..." } else { "" }}</p>
        <p>{move || format!("{:#?}", get_email_login_challenge.value().get())}</p>

        // TODO - Only show the form after the first stage.
        <ActionForm action=answer_email_login_challenge>
            <label for="email">Email challenge response:</label>
            // TODO - Autofill as hidden fields.
            <input
                type="email"
                name="email"
                placeholder="email"
                class="p-1 bg-gray-200 border border-gray-500 invalid:border-red-500"
            />
            <input
                type="text"
                name="challenge"
                placeholder="challenge"
                class="p-1 bg-gray-200 border border-gray-500 invalid:border-red-500"
            />
            <input
                type="text"
                name="response"
                placeholder="response"
                class="p-1 bg-gray-200 border border-gray-500 invalid:border-red-500"
            />
            <input
                type="submit"
                value="Submit challenge"
                class="py-0.5 px-2 bg-green-200 hover:bg-green-300"
            />
        </ActionForm>
    }
}
