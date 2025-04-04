# Updated Rust + HTMX Interaction Rules

This document refines the rules for handling form submissions and responses when using Rust with LocoRS and HTMX, based on recent debugging experiences.

## 1. Distinguish Standard Forms vs. HTMX Forms

*   **Standard HTML Forms (`<form method="POST" action="...">`)**:
    *   **Success:** MUST use a standard HTTP redirect (`axum::response::Redirect::to(...)`). DO NOT use `HX-Redirect` or `HX-Location`.
    *   **Validation/Error:** MUST re-render the *full* page template containing the form, passing back necessary context (e.g., error messages, original form data) to display within the page structure. DO NOT render an error partial directly.
*   **HTMX-Enhanced Forms (`<form hx-post="..." ...>`)**:
    *   **Success (Full Page Navigation):** MUST use the `HX-Redirect: /path/to/redirect` header. This triggers a client-side redirect.
    *   **Success (Partial Update):** If only updating a part of the page, the handler MUST return *only* the HTML fragment needed for the swap. DO NOT render a full page template extending the base layout.
    *   **Validation/Error (Inline Display):**
        *   The form MUST specify `hx-target` (e.g., `#error-message`) and an appropriate `hx-swap` (e.g., `outerHTML`, `innerHTML`).
        *   The handler MUST return *only* the error partial template (e.g., render `error.html` containing the error message). DO NOT render the full form page or use `axum::response::Redirect`.

## 2. Verify Controller Response Matches HTMX Expectation

*   Before marking a feature complete, explicitly check the HTMX attributes (`hx-post`, `hx-target`, `hx-swap`) on the triggering element (e.g., form, button).
*   Verify that the corresponding Rust handler function returns the correct response type:
    *   Full redirect (`Redirect::to` or `HX-Redirect`)?
    *   Full HTML page (`format::render().view("page.html", ...)` extending layout)?
    *   HTML fragment/partial (`format::render().view("partial.html", ...)` *not* extending layout)?

## 3. Email Template Link Generation

*   Links in email templates (both HTML and text) MUST use the `{{domain}}` variable provided by the context.
*   DO NOT hardcode the protocol (`http://` or `https://`) in the template. The `{{domain}}` variable should contain the full base URL including the correct scheme.
    *   **Correct:** `<a href="{{domain}}/path/{{token}}">...</a>`
    *   **Incorrect:** `<a href="http://{{domain}}/path/{{token}}">...</a>`

## 4. Signature Consistency

*   Ensure all user-facing communication (emails, page footers, etc.) uses the correct application signature (e.g., "Your Hosting Farm server") and not default framework placeholders ("The Loco Team"). Check mailer templates.

## 5. Testing Form Submissions

*   Integration/request tests MUST cover both successful submission paths (checking redirects or correct partial swaps) and validation error paths (checking that error messages are rendered correctly in the expected location/format). 