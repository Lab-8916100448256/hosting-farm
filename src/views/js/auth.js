// Authentication handling
const AUTH_TOKEN_KEY = 'auth_token';

// Store the authentication token
export function setAuthToken(token) {
    localStorage.setItem(AUTH_TOKEN_KEY, token);
}

// Get the authentication token
export function getAuthToken() {
    return localStorage.getItem(AUTH_TOKEN_KEY);
}

// Remove the authentication token
export function removeAuthToken() {
    localStorage.removeItem(AUTH_TOKEN_KEY);
}

// Add authentication header to fetch requests
export function addAuthHeader(headers = {}) {
    const token = getAuthToken();
    if (token) {
        return {
            ...headers,
            'Authorization': `Bearer ${token}`
        };
    }
    return headers;
}

// Handle login response
export function handleLoginResponse(response) {
    if (response.token) {
        setAuthToken(response.token);
        return true;
    }
    return false;
}

// Check if user is authenticated
export function isAuthenticated() {
    return !!getAuthToken();
}

// Logout user
export function logout() {
    removeAuthToken();
    window.location.href = '/login';
}

// Make logout available globally
window.logout = logout;

// Intercept fetch requests to add auth header
const originalFetch = window.fetch;
window.fetch = function(url, options = {}) {
    options.headers = addAuthHeader(options.headers);
    return originalFetch(url, options);
}; 