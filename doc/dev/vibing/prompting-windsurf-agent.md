

New feature request:
The profile page should contain 2 additional sections
1. One for the user to configure a list of ssh public key that he/she will use to authenticate on the servers managed by the application 
2. and one for the GPG key that can be used to send him/her crypted emails.

If empty, the GPG key should be automatically retrieved from public gpg servers when the email is verified. At this time the user should be prompted to review his/her GPG key or to configure one if the retrieval from gpg servers failed.

The GPG key section should contain a button to send a verification e-mail. This is similar to the e-mail verification process, except that it sends a crypted email and is used to validate the user's GPG key. This button should be active only if the email has already been verified and a GPG key is configured
