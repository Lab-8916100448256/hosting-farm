# Hosting Farm Web Application Phase 1 Requirements

## Document Information
- **Project Name**: Hosting Farm
- **Version**: 1.0
- **Date**: March 23, 2025
- **Status**: Draft

## Table of Contents
1. [Introduction](#introduction)
2. [User Account Features](#user-account-features)
3. [Team Management Features](#team-management-features)
4. [Access Control System](#access-control-system)
5. [Model](#database-schema)
6. [API Endpoints](#api-endpoints)
7. [UI Components and Pages](#ui-components-and-pages)
8. [Security Considerations](#security-considerations)
9. [Testing Strategy](#testing-strategy)
10. [Implementation Timeline](#implementation-timeline)
11. [Future Enhancements](#future-enhancements)
12. [Appendices](#appendices)


## Introduction

### Purpose
The Hosting Farm application is designed to manage fleets of servers running NixOS.
This document provides comprehensive requirements for the phase 1 of its developement.
Future iterations will expand the functionalities to include hosting infrastructure management features and other operational features.


### Scope
The scope of the phase-1 requirements includes:
- User account management (registration, authentication, profile management)
- Team creation and management
- Role-based access control within teams
- UI design and interaction patterns
- API design and implementation details

### Audience
This document is intended for:
- Development team members
- Project stakeholders
- Quality assurance personnel
- System administrators


### Previous iterations
Refer to the design documents of the previous iterations. Take special attention to phase 0 (project inception) as it contains technical choices and overall requirements that are relevant for all the other phases of the project.
 
- phase 0 - project inception
  - Technology Stack
    - Backend
    - Frontend
    - Development Tools
  - Project Structure and Architecture
    - Architectural Pattern
    - Project Structure
    - Component Diagram
    - Data Flow
      1. Request Handling
      1. Server-Side Rendering with HTMX
  - Security Considerations
    - Authentication Security
    - Authorization Security
    - Data Security
    - Error Handling Security
  - Testing
    - Testing strategy
      - Unit tests
      - Integration tests
      - End-to-end tests
    - Recommended testing tools
  - Best practices


## Technology Stack
As defined in the inception phase, the technology stack used to develop the project is:
### Backend
- **Programming Language**: Rust
- **Web Framework**: loco-rs (Rust on Rails inspired framework)
- **Database**: SeaORM for ORM with PostgreSQL for production environment and SQLite for development environment
- **Authentication**: JWT-based authentication provided by loco-rs
- **Email Service**: SMTP integration for verification emails and notifications

### Frontend
- **UI Approach**: Server-side rendering with HTMX
- **CSS Framework**: TailwindCSS for styling
- **JavaScript**: Minimal usage, primarily for enhancing HTMX functionality
- **Template Engine**: Tera templates for HTML generation

### Development Tools
- **Version Control**: Git
- **Build System**: Cargo (Rust's package manager)
- **Testing Framework**: Rust's built-in testing framework
- **CI/CD**: GitHub Actions or similar


## User Account Features

Some parts of the user account features are already implemented in the skeleton application created from the loco-rs template. These should be re-used and adapted to meet the requirements detailed in this document. 

### User Registration

#### Feature Description
The user registration feature allows new users to create an account in the Hosting Farm application. The registration process includes collecting user information, validating inputs, creating a user record in the database, and sending a verification email.

#### User Interface
The registration page (`/auth/register`) includes:
- A form with fields for name, email, password, and password confirmation
- A submit button to create the account
- A link to the login page for existing users

#### Implementation Details
The registration process follows these steps:
1. Validate user input (email format, password strength, etc.)
2. Check if the email is already registered
3. Hash the password securely
4. Generate an email verification token
5. Create the user record in the database
6. Send a verification email
7. Return a success response

### Email Verification

#### Feature Description
After registration, users receive an email with a verification link. Clicking this link verifies their email address and activates their account.

#### Implementation Details
The verification process follows these steps:
1. User clicks the verification link in the email
2. The system validates the verification token
3. The user's email is marked as verified in the database
4. The user is redirected to the login page with a success message

### User Authentication

#### Feature Description
The authentication feature allows registered users to log in to the application using their email and password. Upon successful authentication, a JWT token is issued for subsequent requests.

#### User Interface
The login page (`/auth/login`) includes:
- A form with fields for email and password
- A "Remember me" checkbox
- A submit button to log in
- Links to the password reset and registration pages

#### Implementation Details
The authentication process follows these steps:
1. Validate user input
2. Find the user by email
3. Verify the password hash
4. Generate a JWT token
5. Return the token and user information


### Password Reset

#### Feature Description
The password reset feature allows users who have forgotten their password to create a new one. It includes requesting a reset link via email and setting a new password.

#### User Interface
1. Forgot Password Page (`/auth/forgot-password`):
   - Form with email address field
   - Submit button to request reset link

2. Reset Password Page (`/auth/reset-password/:token`):
   - Form with new password and confirmation fields
   - Submit button to set new password
   - Password strength indicator

#### Implementation Details
The password reset process follows these steps:
1. User requests a password reset by providing their email
2. The system generates a reset token and sends a reset link via email
3. User clicks the reset link and enters a new password
4. The system validates the token and updates the password
5. User is redirected to the login page

### User Profile Management

#### Feature Description
The user profile management feature allows authenticated users to view and update their profile information, including name and email address.

#### User Interface
The profile page (`/users/profile`) includes:
- Display of current user information
- Form to update profile information
- Option to change password
- List of teams the user belongs to

#### Implementation Details
The profile management includes:
1. Viewing current profile information
2. Updating name and email (with verification for email changes)
3. Changing password (requires current password verification)
4. Viewing team memberships

### Logout

#### Feature Description
The logout feature allows authenticated users to end their session and invalidate their JWT token.

#### Implementation Details
In a stateless JWT system, logout is primarily handled client-side by removing the token from storage. For added security, a token blacklist could be implemented with Redis.

## Team Management Features

### Team Creation

#### Feature Description
The team creation feature allows users to create new teams. When a user creates a team, they automatically become a member of that team with the "Owner" role, which grants them full administrative privileges over the team.

#### User Interface
The team creation page (`/teams/new`) includes:
- A form with fields for team name and description
- A submit button to create the team
- A link back to the teams list

#### Implementation Details
The team creation process follows these steps:
1. Validate team name uniqueness
2. Create the team record in the database
3. Add the creator as a team member with the "Owner" role
4. Redirect to the team details page


### Team Listing and Details

#### Feature Description
These features allow users to view a list of teams they belong to and see detailed information about a specific team, including its members and their roles.

#### User Interface
1. Teams List Page (`/teams`):
   - List of all teams the user is a member of
   - Team name, description, and user's role in each team
   - Create team button

2. Team Details Page (`/teams/:id`):
   - Team name and description
   - List of team members with their roles
   - Team management options based on user's role
   - Invitation management section (for Owners and Administrators)

#### Implementation Details
The team listing and details include:
1. Querying teams the user is a member of
2. Displaying team information and member list
3. Showing appropriate management options based on user's role
4. Handling team editing and deletion (for Owners)

### Team Invitation

#### Feature Description
The team invitation feature allows team Owners and Administrators to invite users to join their team. Invited users receive an email with a link to accept or decline the invitation. When a user accepts an invitation, they are added to the team with the "Observer" role.

#### User Interface
1. Invitation Form (on team details page):
   - Email input field
   - Send invitation button

2. Invitation Email:
   - Team information
   - Accept and decline buttons/links

3. User's Invitations Page (`/users/invitations`):
   - List of pending team invitations
   - Options to accept or decline each invitation

#### Implementation Details
The invitation process follows these steps:
1. Team Owner or Administrator invites a user by email
2. System creates a pending membership record
3. Invitation email is sent to the user
4. User accepts or declines the invitation
5. If accepted, the user is added to the team with the "Observer" role


### Team Member Management

#### Feature Description
The team member management feature allows team Owners and Administrators to manage team members, including changing member roles and removing members from the team. The feature enforces the role-based permissions as specified in the requirements.

#### User Interface
The team member management UI is integrated into the team details page (`/teams/:id`), including:
- List of team members with their roles
- Role selection dropdown for members (based on user's permissions)
- Remove member button (based on user's permissions)

#### Implementation Details
The member management includes:
1. Viewing the list of team members
2. Changing member roles according to permission rules
3. Removing members from the team
4. Enforcing role-based permissions for these actions


### Team Editing and Deletion

#### Feature Description
These features allow team Owners to edit team details or delete the team entirely. Only users with the "Owner" role can perform these actions.

#### User Interface
1. Team Edit Page (`/teams/:id/edit`):
   - Form with team name and description fields
   - Update button
   - Cancel button

2. Team Deletion:
   - Delete button on team details page
   - Confirmation dialog before deletion

#### Implementation Details
The team editing and deletion include:
1. Validating that the user has the "Owner" role
2. Updating team information or deleting the team
3. Redirecting to appropriate pages after the action

## Access Control System

### Role-Based Access Control (RBAC)
The application implements a Role-Based Access Control (RBAC) model where:
- Users are assigned specific roles within each team they belong to
- Each role has a predefined set of permissions
- Access to resources and actions is determined by the user's role
- Role assignments are managed by team Owners and Administrators

### Team Roles Hierarchy
The application defines four distinct roles in descending order of privilege:

1. **Owner**
   - Highest level of access
   - Can perform all actions within a team
   - At least one Owner must exist per team

2. **Administrator**
   - High level of access
   - Can perform most administrative actions
   - Limited in actions affecting Owners

3. **Developer**
   - Medium level of access
   - Can perform development-related actions
   - Limited administrative capabilities

4. **Observer**
   - Lowest level of access
   - Read-only permissions
   - Default role for new team members

### Role Assignment Rules
- When a user creates a team, they automatically become an Owner
- When a user is invited to a team, they initially receive the Observer role
- Only Owners and Administrators can invite users to a team
- Role changes follow specific rules based on the current user's role and the target user's role

### Permission Matrix

| Action                    | Owner | Administrator | Developer | Observer |
|---------------------------|:-----:|:-------------:|:---------:|:--------:|
| Create team               |   ✓   |       ✓       |     ✓     |     ✓    |
| View team details         |   ✓   |       ✓       |     ✓     |     ✓    |
| Edit team details         |   ✓   |       ✗       |     ✗     |     ✗    |
| Delete team               |   ✓   |       ✗       |     ✗     |     ✗    |
| View team members         |   ✓   |       ✓       |     ✓     |     ✓    |
| Invite members            |   ✓   |       ✓       |     ✗     |     ✗    |
| Remove members            |   ✓*  |       ✓**     |     ✗     |     ✗    |
| Change member roles       |   ✓*  |       ✓**     |     ✗     |     ✗    |

*Owner can manage all members except other Owners
**Administrator can only manage Developers and Observers

### Role Change Permissions Matrix

| Current User Role | Target User Current Role | Allowed New Roles for Target User |
|-------------------|--------------------------|-----------------------------------|
| Owner             | Administrator            | Developer, Observer               |
| Owner             | Developer                | Administrator, Observer           |
| Owner             | Observer                 | Administrator, Developer          |
| Administrator     | Developer                | Administrator, Observer           |
| Administrator     | Observer                 | Administrator, Developer          |

Note: Owners cannot change the role of other Owners, and Administrators cannot change the role of Owners.

### Implementation Architecture

The access control system is implemented using middleware that:
1. Authenticates the user based on JWT token
2. Loads the user's team memberships and roles
3. Validates the user's permissions for the requested action
4. Allows or denies the request based on permissions


## Model
The entity for users is already implemented in the skeleton appication created from tje loco-rs template.

Other entities (including database tables, entities classes, database migrations and seeding, CRUD API endpoints, ...)  should be created using the scaffold generation feature of the loco-rs framework (command `cargo loco generate scaffold ...`).
After creation by scaffolding generation the code should be adpated/complemented as needed to fully meet the requirements of this document. 

## API Endpoints

Some of the required API endpoint are already implemented in the skeleton application created from the loco-rs template or will be created by the scaffolding generation of loco-rs. These should be re-used and adapted or complemented as needed to meet the requirements detailed in this document.

### Authentication Endpoints
- `POST /api/auth/register` - Register a new user
- `POST /api/auth/login` - Authenticate a user and return JWT token
- `GET /api/auth/logout` - Invalidate the current session
- `GET /api/auth/verify/:token` - Verify email address
- `POST /api/auth/forgot` - Request password reset
- `POST /api/auth/reset` - Reset password with token

### User Endpoints
- `GET /api/users/me` - Get current user profile
- `PUT /api/users/me` - Update current user profile
- `POST /api/users/me/password` - Change user password
- `GET /api/users/me/teams` - List teams for current user
- `GET /api/users/invitations` - List pending team invitations

### Team Endpoints
- `POST /api/teams` - Create a new team
- `GET /api/teams` - List all teams for current user
- `GET /api/teams/:id` - Get team details
- `PUT /api/teams/:id` - Update team details
- `DELETE /api/teams/:id` - Delete a team

### Team Membership Endpoints
- `POST /api/teams/:id/invitations` - Invite a user to a team
- `POST /api/teams/:id/invitations/accept` - Accept team invitation
- `DELETE /api/teams/:id/invitations/decline` - Decline team invitation
- `GET /api/teams/:id/members` - List all members of a team
- `PUT /api/teams/:id/members/:user_id/role` - Update a member's role
- `DELETE /api/teams/:id/members/:user_id` - Remove a member from a team

## UI Components and Pages

### Authentication Pages
- `/auth/register` - User registration page
- `/auth/login` - User login page
- `/auth/forgot-password` - Request password reset
- `/auth/reset-password/:token` - Reset password form

### User Pages
- `/users/profile` - User profile page
- `/users/teams` - List of user's teams
- `/users/invitations` - List of pending team invitations

### Team Pages
- `/teams` - List all teams
- `/teams/new` - Create a new team
- `/teams/:id` - View team details
- `/teams/:id/edit` - Edit team details
- `/teams/:id/members` - Manage team members
- `/teams/:id/invite` - Invite users to team

### Common Components
- Header with navigation menu
- Footer with links and information
- Error and success message components
- Form components with validation
- Modal dialogs for confirmations

## Security Considerations

### Authentication Security
- Passwords will be hashed using a secure hashing function. This should be already implemented in the skeleton application created from the loco-rs template.
- JWT tokens will have appropriate expiration times
- CSRF protection will be implemented for forms
- Rate limiting will be applied to authentication endpoints

### Authorization Security
- Role-based access control will be enforced at the controller level
- All team operations will verify user membership and role
- API endpoints will validate permissions before processing requests

### Data Security
- Input validation will be performed on all user inputs
- Database queries will use parameterized statements to prevent SQL injection
- Sensitive data will be encrypted at rest

### Error Handling Security
- Error messages will be informative but not reveal sensitive information
- Access control violations will be logged with user ID, IP address, and timestamp
- Repeated violations may trigger temporary account lockouts

## Testing Strategy

### Unit Tests
- Test password hashing and verification
- Test token generation and validation
- Test input validation logic
- Test role validation functions
- Test permission checks

### Integration Tests
- Test registration flow
- Test login and authentication
- Test password reset flow
- Test profile updates
- Test team creation and management
- Test invitation process
- Test role changes with different user roles

### End-to-End Tests
- Complete user registration and verification
- Login with valid and invalid credentials
- Password reset process
- Profile management
- Team creation and management workflow
- Invitation acceptance and rejection
- Role changes and permission enforcement


## Implementation Timeline

### Phase 1: Users, Teams and role-based access control (1 to 4 Weeks?)
- Requirements analysis
- User registration and verification
- Login and session management
- Password reset functionality
- Profile management
- Team creation and management
- Team membership and roles
- Invitation system
- Role-based permission enforcement
- UI templates and HTMX integration
- Comprehensive testing
- Bug fixes and refinements
- Development documentation
- User documentation


## Future Enhancements
Future Enhancements that will be implemented in next development phases

### Server Management Features
- Servers and resources management
- Servers provisioning and configuration
- Servers monitoring and metrics
- Deployment automaion and workflows
- Backup and recovery management
- API for integration with third-party tools

### Advanced Team Features
- Team activity logging
- Team resource usage metrics
- Team billing and quota management
- Enhanced team communication tools

### Security Enhancements
- Multi-factor authentication
- OAuth integration for social login
- Session management across devices
- Enhanced security notifications

### UI Enhancements
- Dark mode support
- Customizable dashboards
- Mobile-responsive design improvements
- Accessibility enhancements

### Technical architecture evolutions
- Horizontal scalability
- Microservice architecture for certain components
- Integration of real-time technologies (WebSockets)
- Multi-region support for high availability


## Appendices

### Glossary
- **loco-rs**: Rust web framework for rapid application development
- **HTMX**: Library allowing access to AJAX features, CSS Transitions, WebSockets, and Server Sent Events directly in HTML
- **SSR**: Server-Side Rendering, an approach where HTML is generated on the server side
- **ORM**: Object-Relational Mapping, a technique for converting between incompatible type systems (here between Rust and SQL)

### References
- Hosting Farm [phase 0 requirements](../phase-0/requirements.md) 
- Official Rust documentation: https://www.rust-lang.org/learn
- loco-rs documentation: https://loco.rs/docs/
- HTMX documentation: https://htmx.org/docs/
