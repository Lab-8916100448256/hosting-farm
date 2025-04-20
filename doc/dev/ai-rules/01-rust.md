---
description: 
globs: 
alwaysApply: true
---
# Rust web application developement rules

## Introduction
This rule file provides guidelines for software development of web application in the Rust language with the loco-rs framework

## Guidelines
1. **UI Flow**
   - When implementing user interfaces always use server-side rendering with HTMX
   - Make minimal usage of javascript, primarily for enhancing HTMX functionality
   - Always prefer using HTMX and Hyperscript over adding custom javascript to the front-end

2  **project structure**
   - the project structure follows the classic loco-rs framework strucure
   - The tera templates for the views are placed in assets/views/
   - loco-rs follows the classic Model View Controller project structure. But keep in mind that the models should contain the logic and operations of the app, not the controllers. The controllers should contain only the code that is specific to API or HTML calls. The actual business logic of the app should be placed in the models. This allow sharing more code between controllers, tasks ar other kind of workers, and not duplicating it everywhere.

3. **project dependencies**
   - Add new dependencies in Cargo.toml only for complex code where adding a new dependencies is really worth it.
   - Do not add any new dependency for anything that is already provided by the loco-rs framework. Look in other places of the code and in the alredy defined dependencies to check if there isn't already something available to implement something before adding a new dependency.
   - You MUST NEVER change any dependencies that are already in Cargo.toml. Doing so would break many things. 
   - Dependencies versions that are set in Cargo.toml MUST NOT be changed! 

3. **HTML Pages generation**
   - Always use Tera templates for producing HTML
   - Use Tailwind CSS for styling.

4. **general coding best practices**
   - When generating code, prioritize readability and maintainability.
   - Use clear and descriptive variable names,
   - avoid complex logic without proper documentation.
   - commit your changes to git regularly

5. **Test and commit after rust code modifications**
   - Rust linter takes time to update after saving a file. To be sure about errors in the code prefer running `cargo check` than relying on linter messages
   - Always run `cargo check` and `cargo test` after modifying or adding Rust code to check for compilation errors and unit test failures. Fix any compilation error and unit test failure that are generated. Then test again. Iterate until there are no more error or unit test failure.
   - unless directly instructed to do so, do not commit the changes to git if there is any compilation error or unit test failure
   - If a test fails ALWAYS assume that the fault is in the tested code, not in the test code or test framework.

6. **Debugging issues that require human interaction with the application**
   - If testing by human interaction is needed to better understand an issue or to verify that a bug fix or new feature implementation works as expected, do not run the application yourself. Ask me to do that and to provide you with the results.

7. **Use loco-rs generate features**
   - To generate a new entity and the associated database migration use `cargo loco generate model ...`. For this to work well you need to pass the new model name and its fileds as parameters. Example : `cargo loco generate model posts title:string! content:text user:references`. Refer to the doc for all the data types : https://loco.rs/docs/the-app/models/
   - To generate a new database migration to alter an existing model use `cargo loco generate migration <name of migration> [name:type, name:type ...]`
   - To generate a new controller use `cargo loco generate controller`
   - To generate a new task use `cargo loco generate task`
   - To generate a new scheduler use `cargo loco generate scheduler`
   - To generate a new worker use `cargo loco generate worker`
   - To generate a new mailer use `cargo loco generate mailer`
   - To generate a full CRUD scaffold with model, controller and API endpoints use `cargo loco generate scaffold`

