/// Helpers related to working with cookies in server functions.
use actix_web::HttpRequest;
use actix_web::cookie::Cookie;
use actix_web::http::header::{HeaderValue, SET_COOKIE};
use leptos::prelude::*;
use leptos_actix::ResponseOptions;

/// Get the ResponseOptions object from leptos_actix.
pub fn use_response_options() -> Result<ResponseOptions, ServerFnError> {
    Ok(use_context::<ResponseOptions>()
        .ok_or_else(|| ServerFnError::new("No response options object"))?)
}

/// Get a cookie as a String.
///
/// To preserve the Option or more easily convert to another type, you can also use request.cookie
/// directly.
pub fn get_cookie(request: &HttpRequest, cookie_name: &str) -> String {
    request
        .cookie(cookie_name)
        .map(|c| c.value().to_string())
        .unwrap_or_default()
}

/// Set headers to set a cookie on the given ResponseOptions object.
pub fn set_cookie(
    response_options: &ResponseOptions,
    cookie: &Cookie,
) -> Result<(), ServerFnError> {
    response_options.append_header(
        SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string())
            .map_err(|err| ServerFnError::new(format!("Failed to encode cookie: {err}")))?,
    );

    Ok(())
}

/// Set headers to delete a cookie on the given ResponseOptions object.
pub fn remove_cookie(response_options: &ResponseOptions, name: &str) -> Result<(), ServerFnError> {
    let mut removal_cookie = Cookie::named(name);
    removal_cookie.set_path("/"); // Otherwise they won't affect most cookies we set.
    removal_cookie.make_removal();

    response_options.append_header(
        SET_COOKIE,
        HeaderValue::from_str(&removal_cookie.to_string())
            .map_err(|err| ServerFnError::new(format!("Failed to encode removal cookie: {err}")))?,
    );

    Ok(())
}
