# Hosting Farm Web Application Phase 0 Requirements

## Document Information
- **Project Name**: Hosting Farm
- **Version**: 1.0
- **Date**: March 22, 2025
- **Status**: Draft

## Table of Contents
1. [Introduction](#introduction)
2. [Technology Stack](#technology-stack)
3. [Project Structure and Architecture](#project-structure-and-architecture)
4. [Security Considerations](#security-considerations)
5. [Testing Strategies](#testing-strategies)
6. [Best practices](#best-practices)
7. [Implementation Timeline](#implementation-timeline)
8. [Future Enhancements](#future-enhancements)

## Introduction

### Purpose
The Hosting Farm application is designed to manage fleets of servers running NixOS.
This document provides comprehensive requirements for the inception phase of the "Hosting Farm" web application. 
Future iterations will expand the functionalities to include user account and team features, role-based access control, hosting infrastructure management and other operational features.


### Scope
The current scope of these requirements includes:
- Presenting the high level technical requirements of the project (developement language, frameworks, best practices)
- Describing the project architecture and the strucure of its files
- Establish best practices that have to be followed during the developement of the project.


### Audience
This document is intended for:
- Development team members
- Project stakeholders
- Quality assurance personnel
- System administrators

## Technology Stack

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

## Project Structure and Architecture

### Architectural Pattern
The application follows the Model-View-Controller (MVC) architectural pattern as encouraged by the loco-rs framework:

1. **Models**: Represent data structures and business logic
2. **Views**: Handle the presentation layer using Tera templates and HTMX
3. **Controllers**: Process requests, interact with models, and return responses

### Project Structure
A git repository should be created on GitHub to version control all the files of the project.
The application will follow the loco-rs framework's conventional project structure for its source code.
A doc folder will be added for development and user documentations:
 
```
hosting_farm/
├── assets/
├── Cargo.lock
├── Cargo.toml
├── config/
├── LICENSE
├── README.md
├── doc/
│   ├── dev/
│   │   ├── phase-0
│   │   │   ├── requirements.md
│   │   │   └── ... other design files
│   │   ├── phase-2
│   │   │   ├── requirements.md
│   │   │   └── ... other design files
│   │   ├── phase-3
│   │   │   ├── requirements.md
│   │   │   └── ... other design files
│   │   └── roadmap.md
│   └── user/
│       ├── install.md
│       ├── admin.md
│       └── user-guide.md
├── migrations/
├── src/
│   ├── app.rs
│   ├── bin/
│   ├── controllers/
│   ├── fixtures/
│   ├── initializers/
│   ├── lib.rs
│   ├── mailers/
│   ├── models/
│   ├── tasks/
│   ├── views/
│   └── workers/
└── tests/
```

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      Client Browser                         │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                      HTTP Server (Axum)                     │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                     Middleware Pipeline                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Logging     │  │ Auth        │  │ Error Handling      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└───────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                        Controllers                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Auth        │  │ Users       │  │ Teams               │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└───┬───────────────────┬───────────────────┬─────────────────┘
    │                   │                   │
    ▼                   ▼                   ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐
│ Models      │  │ Views       │  │ Mailers             │
└─────┬───────┘  └─────┬───────┘  └───────────┬─────────┘
      │                │                      │
      ▼                │                      ▼
┌─────────────┐        │            ┌─────────────────────┐
│ Database    │        │            │ Email Service       │
└─────────────┘        │            └─────────────────────┘
                       ▼
              ┌─────────────────────┐
              │ HTML + HTMX         │
              └─────────────────────┘
```

### Data Flow

1. **Request Handling**:
   - Client sends HTTP request
   - Axum server routes the request to the appropriate controller
   - Middleware processes the request (authentication, logging, etc.)
   - Controller processes the request

3. **Server-Side Rendering with HTMX**:
   - Use of server-side sessions for authentication management
   - User interface state managed primarily on the server side with partial updates via HTMX
   - Controller processes request and prepares data
   - Controller renders HTML using Tera templates
   - HTML with HTMX attributes is sent to client
   - HTMX handles client-side interactions and sends requests back to server
   - Server responds with HTML fragments that HTMX swaps into the DOM


## Security Considerations

### Authentication Security
- Passwords will be hashed using Argon2 or bcrypt
- JWT tokens will have appropriate expiration times
- CSRF protection will be implemented for forms
- Rate limiting will be applied to authentication endpoints

### Authorization Security
- Role-based access control will be enforced at the controller level
- All operations will verify user membership and role
- API endpoints will validate permissions before processing requests

### Data Security
- Input validation will be performed on all user inputs
- Database queries will use parameterized statements to prevent SQL injection
- Sensitive data will be encrypted at rest

### Error Handling Security
- Error messages will be informative but not reveal sensitive information
- Access control violations will be logged with user ID, IP address, and timestamp
- Repeated violations may trigger temporary account lockouts


## Testing

### Testing strategy

#### Unit tests
- Testing models and business logic
- Testing validations and authorization rules
- Target code coverage: >80%

#### Integration tests
- Testing interactions between components
- Testing complete flows (registration, login, team management)
- Testing API and controllers

#### End-to-end tests
- Testing complete user journeys
- Testing the user interface with browser simulation

### Recommended testing tools
- **Unit tests**: cargo test with [Rust's unit testing framework](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html)
- **Integration tests**: cargo test with [Rust's integration testing framework](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html). Use the reqwest crate for testing APIs?
- **End-to-end tests**: Playwright or Cypress


## Best practices
Best practices in software development are essential for creating applications that are efficient, user-friendly, secure, and scalable. 
The folder `doc/dev/best-practices/` contains several overviews of the most important web application development and design best practices, as documented by various sources on internet. (A.I. generated)

The best practices most relevant for the Server Farm project should be extracted from these overviews and listed in a specification document.
Links to detailed informations and resources about each practices should be included.

Example to complete :
### Best practices to apply
#### Development
- Follow Rust code conventions
- Use Rust's type system to prevent errors at compilation
- Document code with rustdoc comments
- Apply SOLID principles
- Use Rust's robust error handling

#### Performance
- Optimize database queries
- Cache frequently accessed data
- Minimize frontend assets (CSS, JS)
- Use ahead-of-time compilation when possible
- Leverage Rust's performance for intensive operations

#### Accessibility
- Follow WCAG 2.1 level AA guidelines
- Ensure keyboard navigation
- Provide alternative texts for visual elements
- Maintain sufficient contrast
- Test with screen readers

#### Security
- Apply the principle of least privilege
- Conduct regular security audits
- Keep dependencies up to date
- Implement a responsible vulnerability disclosure policy
- Train the team on security best practices


## Implementation Timeline

### Phase 0: Inception (2 Days)
- Requirements analysis (this document) (1.5 Days)
- Github repository creation (0.1 Day)
- Project creation from loco-rs template (0.1 Day)
- Readme file and documentation folders structure (0.1 Day)


## Next phases

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

### Phase 2: To Be Defined
- TBD
- TBD
- TBD

## Future Enhancements

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
- Official Rust documentation: https://www.rust-lang.org/learn
- loco-rs documentation: https://loco.rs/docs/
- HTMX documentation: https://htmx.org/docs/
