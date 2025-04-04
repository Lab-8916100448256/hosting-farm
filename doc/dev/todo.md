# Auth Pages TODO Implementation Plan

This plan outlines the steps required to implement the features marked with `TODO` comments in `src/controllers/auth_pages.rs`.

## 1. Implement Forgot Password Functionality

-   [ ] **`handle_forgot_password`**:
    -   [ ] Modify the function to find the user by email (`users::Model::find_by_email`).
    -   [ ] If the user exists:
        -   [ ] Generate a password reset token (`user.generate_reset_token`).
        -   [ ] Save the token and its expiry to the user model.
        -   [ ] Send a password reset email using `AuthMailer::send_forgot_password`.
    -   [ ] Render the `auth/reset-email-sent.html` template regardless of whether the user was found (to prevent email enumeration).
    -   [ ] Add necessary imports (`AuthMailer`, potentially others).
    -   [ ] Create the `assets/views/auth/reset-email-sent.html` template. It should display a confirmation message informing the user to check their email.
-   [ ] **Run Tests & Commit:**
    -   [ ] Run `unset ARGV0 && cargo check` and fix any compilation errors.
    -   [ ] Run `unset ARGV0 && cargo test` and fix any test failures.
    -   [ ] Commit changes with message "feat: Implement handle_forgot_password and reset-email-sent view".

## 2. Implement Reset Password Functionality

-   [ ] **`reset_password`**:
    -   [ ] Modify the function to find the user by the provided reset token (`users::Model::find_by_reset_token`).
    -   [ ] If a user is found and the token is valid (not expired):
        -   [ ] Render the `auth/reset-password.html` template, passing the token to the view.
    -   [ ] If the token is invalid or expired:
        -   [ ] Render the `auth/reset-password.html` template with an error message.
    -   [ ] Add necessary imports.
    -   [ ] Create the `assets/views/auth/reset-password.html` template. It should contain a form with fields for the new password, password confirmation, and the hidden token. It should also conditionally display an error message if the token was invalid. The form should `POST` to a new `handle_reset_password` endpoint.
-   [ ] **`handle_reset_password` (New Endpoint)**:
    -   [ ] Create a new handler function `handle_reset_password` that accepts `Form<ResetPasswordParams>` (a new struct to define).
    -   [ ] Define the `ResetPasswordParams` struct containing `token`, `password`, and `password_confirmation`.
    -   [ ] Add the route `POST /reset-password` pointing to `handle_reset_password` in `auth_pages::routes()`.
    -   [ ] Validate that `password` and `password_confirmation` match. If not, re-render the reset form with an error (`error.html` partial or similar).
    -   [ ] Find the user by the `token` again (`users::Model::find_by_reset_token`).
    -   [ ] If the user is found and the token is valid:
        -   [ ] Update the user's password using the `update_password` method (or similar logic, might need implementation in `users` model).
        -   [ ] Invalidate the reset token.
        -   [ ] Redirect the user to the login page (`/auth/login`) with a success message (e.g., using a query parameter `?reset=success`).
    -   [ ] If the token is invalid or user not found, render an error view (`error.html` or similar).
-   [ ] **Run Tests & Commit:**
    -   [ ] Run `unset ARGV0 && cargo check` and fix any compilation errors.
    -   [ ] Run `unset ARGV0 && cargo test` and fix any test failures.
    -   [ ] Commit changes with message "feat: Implement reset_password view and handle_reset_password endpoint".

## 3. Implement Full Logout Functionality

*Note: Fully invalidating JWTs usually requires maintaining a token blacklist (e.g., in Redis or a database table) or using refresh tokens. A simpler approach for now might be just clearing the client-side token, acknowledging the limitation.*

-   [ ] **`handle_logout`**:
    -   [ ] Research and decide on the JWT invalidation strategy (e.g., blacklist, short expiry + refresh tokens). For this implementation, we'll stick to the client-side clearing as a first step, but add a comment indicating the need for server-side invalidation for full security.
    -   [ ] Keep the current implementation which clears the `auth_token` cookie.
    -   [ ] Add a comment to the function explaining that server-side token invalidation is needed for a complete solution.
-   [ ] **Run Tests & Commit:**
    -   [ ] Run `unset ARGV0 && cargo check` (should pass as no major logic changed).
    -   [ ] Run `unset ARGV0 && cargo test` (should pass).
    -   [ ] Commit changes with message "refactor: Add comment clarifying logout implementation limitation". 