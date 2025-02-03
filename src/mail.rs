use leptos::prelude::*;
use lettre::{
    Message,
    address::Address,
    error::Error,
    message::Mailbox,
    message::{MultiPart, SinglePart, header},
};

pub fn login_code(email_address: Address, code: &str) -> Result<Message, Error> {
    // Create the html we want to send.
    let html = view! {
        <head>
            <title>"Email login/registration code"</title>
            <style type="text/css">
                "* { font-family: Arial, Helvetica, sans-serif; }"
                ".container { display: flex; flex-direction: column; }" ".bigcode {"
                "align-self: center;" "font-family: Courier New, monospace;" "font-size: 200%;"
                "font-weight: bold;" "letter-spacing: 0.2rem;" "margin: 0.2rem auto;" "}"
            </style>
        </head>
        <div class="container">
            <h2>"Email login/registration code"</h2>
            <p>"Hello,"</p>
            <p>"This is an email login code for <site>."</p>
            <p class="bigcode">{code}</p>
            <p>
                "Please go back to the page you requested it from and enter it there. If you did not request this login code, you can ignore it."
            </p>
            <p>"Goodbye."</p>
        </div>
    }.to_html();

    // Plain text fallback.
    let plain_text = format!(
        r#"Email login/registration code

Hello,
This is an email login code for <site>.

{code}

Please go back to the page you requested it from and enter it there. If you did not request this login code, you can ignore it.

Goodbye.
"#
    );

    Message::builder()
        .from("No Reply <noreply@example.com>".parse().unwrap())
        .to(Mailbox::new(None, email_address))
        .subject("Email login/registration code")
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_PLAIN)
                        .body(plain_text),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_HTML)
                        .body(html),
                ),
        )
}
