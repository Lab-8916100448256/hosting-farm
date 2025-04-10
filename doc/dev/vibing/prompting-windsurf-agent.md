# Prompting Windsurf agent

## Exemple prompt 1
Identify all the parts of the rust code where unwrap or the `?` operator are used and write them down as things that need to be fixed in a todo.md file stored in doc/dev/ using markdown format with check-boxes. Do not fix the code. Only create the todo.md file.

=> This did not end well : 
```
Okay, I have created the todo.md file in /home/douzeb/gits/nixin/hosting-farm/doc/dev/.

Because the total number of occurrences of .unwrap() and the ? operator was very large, the file lists the files containing these patterns rather than every single line number. This should give you a good starting point for identifying areas to refactor for more robust error handling.

You can find the file here: doc/dev/todo.md
```


## Exemple prompt 2
try again, limiting the search to files under src/ and working one file at a time to avoid having too many occurences to process at once.
Firt list all the .rs files in the todo.md 
then do the search of unwrap and `?` one file at the time and complete the todo.md file as you go

=> This produced a better result than exemple 1, but the accuracy of the results is still not 100%.

## Exemple prompt 3
When inviting another user in a team, currently, we must select the target user by typing its e-mail, which is not practical if we do not know the exact e-mails of the other users. This needs to be improved.
Add an auto-complete feature that will displays a drop down list to select users based on the first characters that are typed. This must be implemented using an HTMX request that will return the first few matching users to display them in the drop down list, each time some characters are typed. Matching users must be searched by e-mail address and by user name. The request must not be called for each caracters typed if the user is typing quickly. A maximum rate of one request every 200 ms must be observed. 

Followed by :


I tested the new feature. It is working but some improvements can be made : 
- The list contains duplicates. only a unique list should be displayed
- It would be nice if the list diplays both the email and name of the users


## Exemple prompt 4
When inviting another user in a team, currently, we must select the target user by typing its e-mail, which is not practical if we do not know the exact e-mails of the other users. This needs to be improved.
Add an auto-complete feature that will displays a drop down list to select users based on the first characters that are typed. This must be implemented using an HTMX request that will return the first few matching users to display them in the drop down list, each time some characters are typed. Matching users must be searched by e-mail address and by user name. The request must not be called for each caracters typed if the user is typing quickly. A maximum rate of one request every 200 ms must be observed. 
The dropdown list must displays users as "user name (e-mail)" and must not contain duplicates.
Once you have completed the implementation check for any compilation error with `cargo check`. If there are any error fix them and check again. repeat until there are no error left.

=> This did not work well. Was even worse that previous prompte


## Exemple prompt 5
When inviting another user in a team, currently, we must select the target user by typing its e-mail, which is not practical if we do not know the exact e-mails of the other users. This needs to be improved.
Add an auto-complete feature that will displays a drop down list to select users based on the first characters that are typed. This must be implemented using an HTMX request that will return the first few matching users to display them in the drop down list, each time some characters are typed. Matching users must be searched by e-mail address and by user name. The request must not be called for each caracters typed if the user is typing quickly. A maximum rate of one request every 200 ms must be observed. 
The dropdown list must displays users as "user name (e-mail)" and must not contain duplicates.
The main files to change to implement this feature are @teams_pages.rs and  @invite.html 
Do not modify any file with name ending by `_api.rs`

and added workspace rule : 
1. Once you have finished modifying rust code, check for any compilation error with `cargo check`. If there are any error fix them and check again. repeat until there are no error left.