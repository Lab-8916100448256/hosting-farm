[33mcommit 30662c81ac468369a25a2e1c1e3537f7e42404d7[m[33m ([m[1;36mHEAD -> [m[1;32mdev-cursor-13[m[33m)[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 18:27:16 2025 +0200

    feat: send an encrypted password reset email if user has a verified PGP public key confifgured

[33mcommit d08037842193263414161804bf3eb200d944dbac[m[33m ([m[1;31morigin/dev-cursor-13[m[33m)[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 17:51:03 2025 +0200

    echo feat: improve account registration and email verification workflow

[33mcommit f504411ad253d019e833e14067ed3d0a33113dea[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 16:55:39 2025 +0200

    feat: improve profile information modification worlflow

[33mcommit 168d7e9cd5b36839d564ecfd53a36772f01ffdcd[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 10:45:07 2025 +0200

    fix: improve responsiveness of PGP key related buttons

[33mcommit 5089b10da299d1b55a6ae0ff92f702dac4c16d03[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 10:23:16 2025 +0200

    fix: improve styling and fix behaviour of PGP key related buttons  on the profile page

[33mcommit 679254d25e16b2b6b6b66a874528f32f9a731e2a[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 10:12:28 2025 +0200

    feat: asked cursor agent to finish implementing new feature to verify that the application can send PGP encrypted e-mails to a user. I had to help it by givin it an example implementation of PGP encryption and fix some errors manually.

[33mcommit 665d31b66cf091eafce42f2a8b8b60fff12766ca[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 09:29:46 2025 +0200

    feat: asked cursor agent to continue implementing new feature to verify that the application can send PGP encrypted e-mails to a user

[33mcommit a55f28240af8ef7a45e4935a4cbb3722e50cb81b[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Apr 19 09:09:05 2025 +0200

    feat: asked cursor agent, this time with model Gemini 2.5 pro exp, to implement new feature to verify that the application can send PGP encrypted e-mails to a user

[33mcommit 9660a909d5c6cf9df8d37050373ae021852b1fbb[m[33m ([m[1;31morigin/main[m[33m, [m[1;31morigin/dev-codex[m[33m, [m[1;31morigin/HEAD[m[33m, [m[1;32mmain[m[33m)[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Fri Apr 18 07:22:59 2025 +0200

    add codex CLI instructions

[33mcommit 768ace8cc460c12b0adf6c4f0623d8070619cdf1[m[33m ([m[1;33mtag: v0.1.7[m[33m)[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Thu Apr 17 17:02:11 2025 +0200

    merge branch dev-cursor-11 (#15)
    
    * improve cursor rules
    * Asked cursor agent to add the ssh keys features of the profile page. After a few iterations, it is almost working
    * making some progress on the ssh keys features
    * ssh keys feature mostly working. implementation is not that nice though
    * fix: regression in invitation indicator refresh and profile page buttons
    * fix: regression in email verification email (mostly manual modifications, some tab completions used)
    * doc: update comments
    * feat: asked cursor agent to implement a re-send feature for the email verification email
    * fix: correct some issues with the resend email verification feature
    * update cursor rules
    * update cursor rules
    * feat: asked cursor agent ot implement the new feature of having a gpg key in the user profile
    * fix: update tests
    * feat: asked cursor agent to improve the diplay of the GPG key on the profile page, which required to parse teh key and extract its fingerprint and validity time. This did not end well. I reverted all teh code and will try again differently
    * feat: retried asking cursor agent to improve the diplay of the GPG key on the profile page, which required to parse the key and extract its fingerprint and validity time, this time with more guidance in the prompt about the library to use. It did not manage to do it either. I had to give it a sample code for the PGP key parsing for it to complete its task.
    * fix: correct a few issues with the new PGP key feature of the profile page
    * typo: fix message
    * release: bump version to 0.1.7
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit 789e0a3d18ee47e616bc472f11e1d5847ce87eef[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Wed Apr 16 05:30:34 2025 +0200

    improve cursor rules

[33mcommit 796b8987b7b36ba9b87305717ef9ed0fdad14856[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Tue Apr 15 20:03:24 2025 +0200

    typo: fix some typos in cursor rules

[33mcommit d8785a52c45bce33293c4f1641162c4d4fa69db1[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Tue Apr 15 19:30:53 2025 +0200

    merge branch dev-cursor-9 (#14)
    
    * asked cursor agent to review the code of some controller to find unwraps and question mark operator usage
    * asked cursor agent to change the copyright notice in the footer by a copyleft one. But it produced a bad copyleft symbol. I had to replace it manually by the one from wikimedia.
    * refact: replace usage of unwrap by more idomatic let if constructs
    * refact: unify redirects by using utility function that performs htmx or http redirect depending on context
    * refact:improve some errors handling in auth_pages controller
    * refact: improve some errors handling in user_pages controller
    * refact: refactoring of render_template()
    * refact: improve some errors handling in treams_pages controller
    * refact: manual code cleanup and review cursor rules
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit 5f69b9adbb34b4870d1f4117c95e469e72438c6b[m[33m ([m[1;33mtag: v0.1.6[m[33m)[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Sun Apr 13 09:13:48 2025 +0200

    Dev cursor 8 (#13)
    
    * feat: Asked cursor agent to implement some small improvements to the team invite page auto-complete feature.
    * bump version to 1.0.6
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit fd9541a45c282613ab7f56ae3f640e5b5235a7e4[m[33m ([m[1;33mtag: v0.1.5[m[33m)[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Sat Apr 12 10:00:32 2025 +0200

    merge branch dev-cursor-7 (#11)
    
    * feat: asked cursor agent to add an auto complete feature to the team invite page (using gemini 2.4 pro exp 03-25)
    * fix: asked cursor agent to fix compilation errors
    * fix: asked cursor agent to fix the issue of the auto-complete dropdown list not showing up
    * fix: ask cursor agent to fix the team invite form submission that it broke when fixing the issue of the auto-complete feature
    * fix: asked cursor agent to fix the issue of the auto-complete dropdown list that was broken by the previous fix of the form submission
    * fix: ask cursor agent to revert the label text to "Email address" and the input type back to "email"
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit 99df85e30deba71ed0053aa63be9a657bb0c1cf4[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Thu Apr 10 22:46:29 2025 +0200

    manual change

[33mcommit 0a129aea87c698e932377840e40284b5b64cf088[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Thu Apr 10 22:45:57 2025 +0200

    manual changes

[33mcommit 28af89dec8ceabb819b3f4d1770a7a4af113512f[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Thu Apr 10 14:47:31 2025 +0200

    merge brancg dev-cursor-6 (#10)
    
    * fix: redirect to hame page from /auth/register is user is already authenticated
    * feat: Error 404 fallback page
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit 37b9bf1db89db3c89a2f7b3cbe5b9d64c8b14170[m[33m ([m[1;33mtag: v0.1.4[m[33m)[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Tue Apr 8 22:51:16 2025 +0200

    merge dev-cursor-5 (#9)
    
    * fix: correct email messages (links, line breaks)
    * feat: asked agent with gemini 2.5 pro exp 03-25 to update the invitation count when accepting or denying an invitation and to also trigger an update on a regular basis
    * fix: asked agent with gemini 2.5 pro exp 03-25 to correct the login page redirect when the user is already authenticated
    * fix: fix: asked agent with gemini 2.5 pro exp 03-25 to correct all the authenticated pages to make them redirect to the login page if the user is not authenticated.
    * fix: review agent code of previous commit and clean-up the mess it did.
    * release: v0.1.4
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit 85f13c59c9d75d4ff1f6e5522371fef79fe613f9[m[33m ([m[1;33mtag: v0.1.3[m[33m)[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Mon Apr 7 12:04:25 2025 +0200

    release: v0.1.3

[33mcommit 782fee839ea325bc1055db3280b7df3e001c5eeb[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Mon Apr 7 09:35:47 2025 +0200

    merge dev-curso-4 branch (#8)
    
    * fix: manually fix team invitation workflow and some code cleanup
    
    * Add extention recommendations
    
    * feat: better error messages on the team invite page (mostly manual edit)
    
    * fix: fix url in emails and improve lots of error cases handling. Maybe still some work to do, need to continue code review. No agent used, but did a lot of cursor tab
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit b7c80eed6fae9a941f87543476beb68e6add8226[m[33m ([m[1;33mtag: v0.1.2[m[33m)[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Fri Apr 4 08:46:03 2025 +0200

    build check and unit test of new release

[33mcommit fa95a011136a51f81a79b8fda5371f9c4fdbf9b0[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Fri Apr 4 08:25:12 2025 +0200

    Update Cargo.toml
    
    release: increment migration version to 0.1.2

[33mcommit 48faea67b48b8ae6e586229817645134b627f030[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Fri Apr 4 08:23:52 2025 +0200

    release: increment version to 0.1.2

[33mcommit 176b2d87918fe602601b9ff8b6e12487142bcf2f[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Fri Apr 4 03:31:08 2025 +0200

    Merge branch cursor-dev-3 : Dev session with Cursor agent, using Gemini 2.5 Pro experimental 03-25
    
    * feat: Implement handle_forgot_password and reset-email-sent view
    * feat: Implement reset_password view and handle_reset_password endpoint
    * refactor: Add comment clarifying logout implementation limitation
    * fix: Correct forgot/reset password flow and email templates. Fixes confirmation page rendering (redirect), email content/link, success redirect (HX-Redirect), and error display (HTMX swap).
    ---------
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit d0cc73933d17dda32285d9da101f8536942f9d07[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Thu Apr 3 20:38:48 2025 +0200

    Merge dev-curso-2 into main (#6)
    
    * refactoring: update to loco_rs v0.15 adn start manually cleaning-up the mess generated by Cursor and improving rules to improve future codegen
    
    * refactoring: continue manual cleang-up of the mess generated by Cursor
    
    * refactoring: continue manual cleaning-up of the mess generated by Cursor
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>

[33mcommit bdc7d3a8473ded9269894d0f42c0d50ffc26164b[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Thu Apr 3 19:56:25 2025 +0200

    typo: remove duplicated and empty setting

[33mcommit 0d6bef4d651274129bb3f7c6b51d54c9e597793a[m[33m ([m[1;33mtag: v0.1.1[m[33m)[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Sat Mar 29 20:25:52 2025 +0100

    Improve NixOS packaging (#4)
    
    * feat: NixOS support, handle assets files - First default.nix proposal by gemini 2.5 Pro
    
    * feat: NixOS support, handle assets files - Second proposal by gemini 2.5 Pro
    
    * debug: add log for debugging nixos package
    
    * debug: add more log for debugging nixos package
    
    * debug: add more log, with the help of gemini, for continuing debugging nixos package
    
    * feat: NixOS support, handle assets files - New proposal by gemini 2.5 Pro
    
    * debug: Fix a couple of isses with the configuration.nix example
    
    * fix: securing the /etc/hosting-farm/production.yaml configuration file
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <12b@distrilab.fr>

[33mcommit 1019d5782395cb8f048939b411202bb7c7d857f8[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Fri Mar 28 21:56:43 2025 +0100

    Marge dev-cursor into main (#3)
    
    * Implementation of phase-1 requirements with Cursor, first try
    using agent mode with thinking option and model claude-3.7-sonnet
    
    * Fix team mailer functionality and route paths format (compilation and unit test errors)
    
    * Add team mailer implementation chat history to documentation
    
    * Replace the partial chat history generated by cursor agent by a better one generated with the specstory extension
    
    * renamed cursor chat history file to avoir confusion with manus chat history if we ever merge the 2 branches at some point (unlikely though)
    
    * Add UI templates for phase 1 requirements
    
    * Add controller routes for UI templates
    
    * Fix controller templates rendering and authentication middleware
    
    * Asked cursor agent to fix all compilation errors.
    Agent summary of what was done:
    Great! Our changes have successfully fixed the compilation issues. Let's summarize what we did:
    We fixed the middleware.rs file by implementing a placeholder function that provides clear documentation on how to use authentication in the application. The JWT authentication in loco-rs works directly through the function parameter extraction pattern, so we don't need to implement complex middleware.
    We fixed the template.rs file by creating a direct implementation that uses Tera without going through the ViewEngine trait. This approach bypasses the issues with access to the AppContext extensions.
    We updated route definitions in the controllers to use the JWT parameter for authentication directly rather than trying to use middleware.
    These changes have successfully fixed the compilation errors, and the application should now compile correctly. The approach we took is pragmatic and works with the current API structure of loco-rs.
    
    * Update route parameters to use curly braces format for axum 0.8 compatibility
    
    * Fix Tera template conditional syntax in team and invitation templates
    
    * Fix ternary operator in teams/show template with proper Tera conditional syntax
    
    * Fix form submission and client-side redirects for login and registration
    
    * Fix form submissions to use JSON content type
    
    * Revert "Fix form submissions to use JSON content type"
    
    This reverts commit a014494aa474ede0e2cce286d5d72bbf4fb2366d.
    
    * Add form handlers in auth_pages to handle form submissions with HTMX
    
    * Fix compiler errors in auth_pages.rs handling
    
    * Fix mailer to explicitly set the from email address
    
    * ignore config files with sensitive informations and replace them with example files
    
    * Make email from address configurable via SMTP user config
    
    * Add HTML page for email verification result
    
    * Separate API and human-facing email verification endpoints
    
    * Move home page to home_pages.rs to follow endpoint rules
    
    * fix: make home page handle both authenticated and unauthenticated users
    
    * fix: use proper auth::JWT for home page authentication
    
    * End of cursor session. Commit last code changes, add some cursor rules, add session chat history.
    
    * doc: move cursor chat history
    
    * fix: fixed several issue with the login workflow using cursor agent, but he forgot the rule that instruted him to make regular commit by himself
    
    * fix: improve user menu interaction and styling
    
    * update cursor rules
    
    * debug: manual edits for easier debug of issue with user drop down menu
    
    * add some cursor agent chat history
    
    * fix: fixed the issue with the user dorpdown menu with Cursor agent after some manual debug of the code to understand what was going on and be able to hint the agent in the right direction. Because it was doing all kind of shit when trying to fix the issue by itself. (See previous chat history)
    
    * feat: implement the logout button with cursor. working but it made some nonsense when I asked to redirect to the unauthenticated home page when trying to access to the authenticatd home page without being logged in. And I've had enough nonsense for today. So I commit all this shit.
    
    * cursor: update rules to alwaysApply mode
    
    * fix: update teams routes order to handle /new path correctly and add invitation count to create team page
    
    * update cursor rules
    
    * chore: commit files that cursor agent forgot to commit
    
    * doc: add some cursor chat history
    
    * fix: Update team creation form to use human-facing endpoint instead of API endpoint
    
    * fix: explicitly set team pid during creation to avoid race condition
    
    * Fix team creation and team details page error
    
    * fix: update team_details handler to use correct show.html.tera template
    
    * fix: add missing variables to team details template context
    
    * doc: Add some cursor agent chat history. Unfortunately, I have hit the free plan limit for premium models fast queries.
    
    * fix: add missing invitation_count to team_details page and fix show.html.tera template structure
    
    * Fix: Add missing invitation_count to template contexts
    
    * Fix: Add invitation_count to authenticated pages
    
    * debug: Manually improve tera templates error log to improve debug of template renderig failures
    
    * fix: End of session with Cursor agent - fixing issues with team related pages templates
    
    * cursor: Rework Cursor rules
    
    * feat: implement the user profile modifications features (with cursor agent and a little bit of a human touch to correct the fact that it is still trying to use api endpoint for human-facing stuff depite the detailes rules I have given to him
    
    * feat: implement the team edit feature using cursor agent.
    
    * Add redirect to teams list page after team deletion
    
    * Refactor users controller to users_pages according to endpoint naming conventions
    
    * refactor: remove the old users controller that Cursor agent forgot to delete when renaming it
    
    * feat: Implement some missing features and refactor some endpoints using Cursor Agent (and a litle bit of human finessing)
    
    * feat: Finish to implement and debug the teams features (invitations and roles changing) with Cursor agent
    
    * refactor: manually refactor one format::redirect into a HTMX redirect
    
    * refactor: refactor more `format::redirect` into HTMX redirect, using Cursor agent and giving him the manual exammle I just did. prompt below:
    change all the controller handlers that are performing a redirect using `format::redirect` to use an HTMX redirect, as in this exemple in diff format:
    ```diff
         // Redirect to the team details page
    -    format::redirect(&format!("/teams/{}", updated_team.pid))
    +    let response = Response::builder()
    +        .header("HX-Redirect", format!("/teams/{}", updated_team.pid))
    +        .body(axum::body::Body::empty())?;
    +
    +    Ok(response)
     }
    ```
    
    * fix: manually fix some redirect and use agent to fix all unused imports warnings
    
    * feat: use agent to display an error when a user cannot be invited to a team
    
    * feat: Use Cursor agent to add the invited users to the team member list and implment an invitation revocation feature
    
    * style: use Cursor Agent to adjust team member list layout
    
    * doc: add some screenshots of what happened while doing changes of the previous commit
    
    * style: use Cursor Agent to adjust team member list layout (part 2, forgot to commit this file)
    
    * feat: NixOS support
    
    * fix: Asked Cursor agent to fix the home page link at the top left-hand corner of the views that was always redirecting to the unauthenticated home page.
    
    * feat: Asked to Cursor Agent to implement the "remember me" feature of the login page and to automatically redirect to the home page if the user is already authenticated. Prompt below
    Make is so that when the user has checked this checkbox when logging-in, a never expiring cookie is set to remember its email. And then, when the login view is displayed, if this cookie exists, the e-mail field should be automatically be set to the value stored in that cookie.
    Also, if the user is already authenticated, the login view should should not be displayed. Instead the user should be automatically redirected to the authenticated home view (/home)
    
    * fix: correct login page link on email verification view and profile menu item text
    
    * feat: NixOS support - 2nd try
    
    * NixOS support - 3rd try
    
    * NixOS support - 4th try
    
    * NixOS support - 5th try
    
    * NixOS support - 6th and final try!!!
    
    * doc: improve nixos build documentation
    
    ---------
    
    Co-authored-by: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
    Co-authored-by: Fabrice Bellamy <12b@distrilab.fr>

[33mcommit f22ee21472da8edfa8ea8b7d1dee21c90766bb2b[m[33m ([m[1;31morigin/dev-human[m[33m)[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Mon Mar 24 19:33:49 2025 +0100

    doc: typo

[33mcommit 5852f71b2405e8ed1533d18b3cd03e228b6bd62b[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sun Mar 23 23:01:46 2025 +0100

    doc: Duplicate the technology stack chapter from phase-0 requirement into phase-1 requirements because the first implementation test with manus.ai got it confused. It implemented a client-side rendered front-end

[33mcommit 7bda3d8f71d2f78fc175774848162f1ecbd63cb2[m
Merge: 8ccd21d 0894c36
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Sun Mar 23 22:42:06 2025 +0100

    Merge pull request #2 from Lab-8916100448256/dev-human
    
    Remove unused client-side rendered front-end

[33mcommit 0894c3604fb05dbdac19ebd11b55fac60c036df5[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sun Mar 23 22:37:29 2025 +0100

    Remove unused client-side rendered front-end

[33mcommit 8ccd21deaf806b8ce2a0406841889e7e91953840[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sun Mar 23 20:09:18 2025 +0100

    doc: add phase 1 detailed requirements

[33mcommit 8c13848324b25ce9c48037b007c92d2b023ba192[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sun Mar 23 20:08:07 2025 +0100

    doc: typo

[33mcommit a12a37e922268cb22b846390a3af95773b9f8388[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sun Mar 23 15:34:43 2025 +0100

    Add another research results about web developement best practices

[33mcommit 2548db982aff23ed3ed2f7462f54c2a8a6b35191[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sun Mar 23 15:19:39 2025 +0100

    Add another research results about web developement best practices

[33mcommit 8ec9f04c5a9bfc325561bb91ffbf6a32af1f6a30[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sun Mar 23 14:34:25 2025 +0100

    Add more research results about web developement best practices

[33mcommit dcdd6aa2e40d065f901e5545c41e744a717d6719[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sun Mar 23 00:13:51 2025 +0100

    ignore sqlite temp files

[33mcommit 9dd0e5671bbc2655500379043399cb31f4af41e8[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sat Mar 22 23:42:22 2025 +0100

    add documentation structure and phase 0 requirements documentation

[33mcommit dad17e02464df609e7d69db787ae77ad3ad40453[m
Author: Fabrice Bellamy <fabrice.bellamy@distrilab.fr>
Date:   Sat Mar 22 16:45:21 2025 +0100

    Create application skeleton from loco-rs template "Saas App with server side rendering", using command `loco new` with options `--db sqlite --bg async --assets serverside`

[33mcommit 27b61de831342881954b05ca9e1f84bff9012e26[m
Author: Fabrice Bellamy <12b@distrilab.fr>
Date:   Sat Mar 22 16:44:05 2025 +0100

    Update readme

[33mcommit 4f0ed670947fd2bded5edd49df49632cb0afef34[m
Author: Lab-8916100448256 <38373466+Lab-8916100448256@users.noreply.github.com>
Date:   Fri Mar 21 13:00:55 2025 +0100

    Initial commit
