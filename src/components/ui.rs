/// Common UI building blocks.
use leptos::prelude::*;
use leptos_router::components::*;

/// Normal link.
#[component]
pub fn ANorm<H>(href: H, children: Children) -> impl IntoView
where
    H: ToHref + Send + Sync + 'static,
{
    view! {
        <span class="text-blue-600 hover:text-blue-400 hover:underline *:aria-[current='page']:underline">
            <A href=href>{children()}</A>
        </span>
    }
}
