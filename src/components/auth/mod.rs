/// All authentication-related components and server functions.
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::*;

mod email;
mod login;
mod register;

/// Visual wrapper around all auth views, but there isn't much to show.
#[component]
fn AuthWrapper() -> impl IntoView {
    view! {
        <h1 class="mb-2 text-4xl font-bold">"Login/register"</h1>
        <Outlet />
    }
}

/// Route definitions for /auth subtree.
#[component(transparent)]
pub fn AuthRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("auth") view=AuthWrapper>
            <Route path=path!("") view=login::LoginMethods />
            <email::Routes />
            <register::Routes />
            <Route path=path!("register") view=register::Register />
        </ParentRoute>
    }
    .into_inner()
}
