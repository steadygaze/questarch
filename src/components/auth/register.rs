use crate::components::ui::*;

use codee::string::FromToStringCodec;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::{MatchNestedRoutes, path};
use leptos_use::use_cookie;

#[cfg(feature = "ssr")]
mod ssr {
    pub use crate::app_state::*;
    pub use crate::cookie::*;

    pub use actix_web::HttpRequest;
    pub use leptos_actix::extract;
    pub use sqlx::Executor;
    pub use sqlx::Row;
    pub use uuid::Uuid;
}

/// Route definitions for registration stages.
#[component(transparent)]
pub fn Routes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("register") view=RegisterContainer>
            <Route path=path!("") view=Register />
            <Route path=path!("cancel") view=Cancel />
        </ParentRoute>
    }
    .into_inner()
}

/// Fetch the HTTP-only cookie.
#[server]
async fn register_new_user(
    create_profile: Option<String>,
    display_name: Option<String>,
    username: Option<String>,
    bio: Option<String>,
) -> Result<(), ServerFnError> {
    use self::ssr::*;

    let request: HttpRequest = extract().await?;
    let email = get_cookie(&request, "regmail");
    let code = get_cookie(&request, "regcode");

    if email.is_empty() || code.is_empty() {
        // TODO - Additional checks: valid email and code.
        return Err(ServerFnError::new("Params are missing or invalid"));
    }

    let app_state = use_app_state()?;

    // Create a transaction for both creating the account and the profile.
    let mut transaction = app_state.db_pool.begin().await.or_else(|err| {
        Err(ServerFnError::new(format!(
            "Failed to create transaction for account creation: {err}"
        )))
    })?;

    let id: Uuid = transaction
        .fetch_one(
            sqlx::query(
                r#"
                insert into account (email)
                values ($1)
                returning id
                "#,
            )
            .bind(&email),
        )
        .await
        .or_else(|err| {
            Err(ServerFnError::new(format!(
                "Failed to create transaction for account creation: {err}"
            )))
        })?
        .get(0);

    if create_profile.is_some_and(|c| c == "true") {
        let profile_id: Uuid = transaction
            .fetch_one(
                sqlx::query(
                    r#"
                    insert into profile (username, account_id, display_name, bio)
                    values ($1, $2, $3, $4)
                    returning id
                    "#,
                )
                .bind(&username)
                .bind(id)
                .bind(&display_name)
                .bind(&bio),
            )
            .await
            .or_else(|err| {
                Err(ServerFnError::new(format!(
                    "Failed to create transaction for account creation: {err}"
                )))
            })?
            .get(0);

        if transaction
            .execute(
                sqlx::query(
                    r#"
                    update account
                    set default_profile = $1
                    where id = $2
                    "#,
                )
                .bind(profile_id)
                .bind(id),
            )
            .await
            .or_else(|err| {
                Err(ServerFnError::new(format!(
                    "Failed to create transaction for account creation: {err}"
                )))
            })?
            .rows_affected()
            <= 0
        {
            return Err(ServerFnError::new("Failed to find account to update"));
        }
    }

    transaction.commit().await.or_else(|err| {
        Err(ServerFnError::new(format!(
            "Failed to commit account creation: {err}"
        )))
    })?;

    let response_options = use_response_options()?;

    remove_cookie(&response_options, "regcode")?;
    remove_cookie(&response_options, "regmail")?;

    app_state
        .create_session(&response_options, id, username, display_name)
        .await
        .or_else(|err| {
            Err(ServerFnError::new(format!(
                "Failed to create new session after account creation: {err}"
            )))
        })?;

    leptos_actix::redirect("/");

    Ok(())
}

#[component]
pub fn RegisterContainer() -> impl IntoView {
    view! {
        <fieldset class="px-2 pt-1 pb-2 border-2 border-slate-500">
            <legend class="m-2 text-2xl font-bold">Register</legend>
            <Outlet />
        </fieldset>
    }
}

#[component]
pub fn Register() -> impl IntoView {
    let register_new_user = ServerAction::<RegisterNewUser>::new();

    let (email, _) = use_cookie::<String, FromToStringCodec>("regmail");

    let create_profile = RwSignal::new(false);
    let bio = RwSignal::new("".to_string());

    view! {
        <Show
            when=move || email.get().is_some()
            fallback=move || {
                view! {
                    "You need to "
                    <ANorm href="/auth/email">"verify your email"</ANorm>
                    " before registering."
                }
            }
        >
            <p>
                "You don't have an account yet, or haven't associated this email with your account. Fill in this form to create a new account, or login with your previous email first to associate this email with your existing account."
            </p>

            <ActionForm action=register_new_user>
                <fieldset class="p-2 my-2 border-2 border-slate-500">
                    <legend class="text-xl font-bold">Account</legend>
                    <div class="pb-2">
                        <label for="email">"Email: "</label>
                        <input
                            type="email"
                            name="email"
                            id="email"
                            value=move || email.get().unwrap_or_default()
                            class="border-2 border-slate-100"
                            // Doesn't need to be re-sent.
                            disabled
                        />
                        <p>
                            "To change the email you register as, "
                            <ANorm href="/auth/email">"verify with a different email"</ANorm>
                            " first".
                        </p>
                    </div>
                    <div class="py-2">
                        <section class="overflow-y-scroll p-2 mb-4 h-64 border-2 border-slate-500">
                            <h1 class="text-2xl font-bold">Terms of Service</h1>

                            <p>
                                Lorem ipsum dolor sit amet, consectetur adipiscing elit. Praesent
                                cursus turpis neque, sed dapibus mauris pretium sit amet. Aliquam
                                sed justo ut felis interdum scelerisque. Vivamus id volutpat augue.
                                Vivamus sed augue id augue varius vestibulum. Cras ullamcorper purus
                                at porttitor cursus. Sed elit elit, accumsan at pulvinar nec, cursus
                                accumsan libero. Donec euismod nunc in ipsum tempor bibendum. Etiam
                                scelerisque, nunc eu auctor tincidunt, erat purus volutpat arcu,
                                eget molestie quam urna nec lacus. Proin mauris nisi, pellentesque
                                eget volutpat et, tincidunt maximus diam. Donec vitae suscipit diam.
                            </p>

                            <p>
                                Nullam ultricies egestas suscipit. Etiam sit amet ultricies libero.
                                Nam neque purus, ultrices at dignissim blandit, tincidunt eget quam.
                                Praesent vel leo vel eros iaculis aliquam vitae sit amet est.
                                Quisque tincidunt sem quis orci aliquam convallis. Vivamus eu purus
                                eget neque egestas maximus at vitae nisi. In hac habitasse platea
                                dictumst. Vestibulum tristique dui nulla, et commodo ex efficitur
                                id. Phasellus dapibus feugiat congue. Aliquam viverra euismod
                                lectus, a placerat leo ornare at. Mauris quam neque, sollicitudin
                                vitae eros ut, ultricies tincidunt odio. Fusce vestibulum enim
                                laoreet dui hendrerit, sed efficitur sapien ornare.
                            </p>

                            <p>
                                Etiam et elit at libero euismod mattis vel quis nunc. Suspendisse
                                potenti. Nam ac ex nisi. Mauris facilisis molestie libero, et
                                suscipit ex suscipit quis. Fusce imperdiet libero nulla, sed semper
                                diam sodales sit amet. Sed viverra ut nulla quis fringilla. Nunc id
                                malesuada quam, eget tincidunt mauris. Duis vel suscipit risus, non
                                maximus tellus.
                            </p>
                        </section>
                        <label for="tos_ack">"Agree to terms of service: "</label>
                        <input type="checkbox" id="tos_ack" name="tos_ack" required />
                        <p>
                            "You must agree to the terms of service to proceed. "
                            // TODO - Actually make a page for the TOS.
                            <ANorm href="/">"(Open in a new tab.)"</ANorm>
                        </p>
                    </div>
                    <div class="pb-2">
                        <label for="create_profile">"Create profile: "</label>
                        <input
                            type="checkbox"
                            name="create_profile"
                            id="create_profile"
                            bind:value=create_profile
                        />
                        <p>
                            "You can create a full profile now or later. A profile allows interaction and posting. If you don't intend to vote or post publicly, you can skip the next section."
                        </p>
                    </div>
                </fieldset>
                <fieldset
                    class="p-2 my-2 border-2 border-slate-500"
                    class=("opacity-50", move || !create_profile())
                    disabled=move || !create_profile()
                >
                    <legend class="text-xl font-bold">Profile</legend>
                    <div class="py-2">
                        <label for="display_name">"Display name: "</label>
                        <input
                            type="text"
                            name="display_name"
                            id="display_name"
                            placeholder="Display name"
                            maxlength="30"
                            autocomplete="off"
                            class="p-0.5 border-2 border-slate-300 disabled:border-slate-100"
                        />
                        <p>
                            "This text is displayed alongside your username for others to easily identify you. Up to 30 characters long."
                        </p>
                    </div>
                    <div class="py-2">
                        <label for="username">"Username: "</label>
                        <input
                            type="text"
                            name="username"
                            id="username"
                            placeholder="Username"
                            required
                            minlength="3"
                            maxlength="20"
                            pattern="[a-z][a-z0-9]*"
                            autocomplete="off"
                            class="p-0.5 border-2 border-slate-300 disabled:border-slate-100"
                        />
                        <p>
                            A username is also used to identify you, but may be abbreviated
                            compared to the display name. It may be used by others to refer to
                            you. It may contain only letters or numbers, be between three and
                            thirty characters long, and must start with a letter.
                        </p>
                    </div>
                    <div class="py-2">
                        <p>
                            <label for="bio">Bio:</label>
                        </p>
                        <textarea
                            name="bio"
                            id="bio"
                            placeholder="Bio"
                            maxlength="500"
                            autocomplete="off"
                            class="p-0.5 w-full border-2 border-slate-300 disabled:border-slate-100"
                            bind:value=bio
                        ></textarea>
                        <p>
                            "("<span id="bio-char-counter">{move || bio.get().len()}</span>
                            "/500 characters)"
                        </p>
                    </div>
                </fieldset>
                <div class="py-2">
                    <input
                        class="py-0.5 px-2 mr-1 font-bold bg-green-200 hover:bg-green-400"
                        type="submit"
                        value="Create account"
                    />
                    <input
                        class="py-0.5 px-2 font-bold bg-slate-200 hover:bg-slate-400"
                        type="submit"
                        formaction="/auth/register/cancel"
                        value="Cancel"
                        // Can cancel without checking TOS checkbox.
                        formnovalidate
                    />
                </div>
            </ActionForm>
        </Show>
    }
}

#[server]
async fn cancel_registration() -> Result<(), ServerFnError> {
    use crate::cookie::*;
    let response_options = use_response_options()?;
    remove_cookie(&response_options, "regcode")?;
    // TODO - Delete server saved registration info.
    Ok(())
}

#[component]
pub fn Cancel() -> impl IntoView {
    let (_, set_email) = use_cookie::<String, FromToStringCodec>("regmail");

    set_email(None);

    view! {
        <Await future=cancel_registration() let:_>
            <p>
                "Registration cancelled. If you are not automatically redirected, you can navigate away from this page now."
            </p>
        </Await>
    }
}
