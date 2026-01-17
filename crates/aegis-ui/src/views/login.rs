//! Login/authentication view.

use dioxus::prelude::*;

use crate::state::AppState;
use crate::components::icons::ShieldIcon;

/// Login view component.
#[component]
pub fn LoginView() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let error_msg = state.read().error_message.clone();

    rsx! {
        div { class: "auth-container",
            // Logo
            div { class: "auth-logo",
                ShieldIcon { class: Some("auth-logo-icon".to_string()) }
                h1 { class: "auth-logo-title", "Aegis" }
                p { class: "auth-logo-subtitle", "AI Safety for Families" }
            }

            // Login card
            div { class: "auth-card",
                h2 { class: "auth-card-title", "Enter Password" }

                form {
                    class: "auth-form",
                    onsubmit: move |evt| {
                        evt.prevent_default();
                        attempt_login(&mut state);
                    },

                    input {
                        class: "input",
                        r#type: "password",
                        placeholder: "Password",
                        value: "{state.read().password_input}",
                        oninput: move |evt| {
                            state.write().password_input = evt.value();
                        }
                    }

                    button {
                        class: "btn btn-primary btn-lg w-full",
                        r#type: "submit",
                        "Unlock"
                    }
                }

                // Error message
                if let Some(error) = error_msg {
                    div { class: "auth-error mt-md", "{error}" }
                }
            }
        }
    }
}

/// Attempts to login with current password.
fn attempt_login(state: &mut Signal<AppState>) {
    let password = state.read().password_input.clone();
    let result = state.write().login(&password);
    if let Err(e) = result {
        state.write().set_error(e.to_string());
    }
}
