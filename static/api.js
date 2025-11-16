/**
 * API utility for consistent fetch handling
 */

const API_BASE = window.location.origin;

/**
 * Make an API request
 * @param {string} endpoint - API endpoint (e.g., '/posts')
 * @param {Object} options - Fetch options
 * @param {string} options.method - HTTP method (default: 'GET')
 * @param {Object} options.body - Request body (auto-stringified)
 * @param {string} options.token - Auth token (auto-added as Bearer)
 * @param {boolean} options.json - Parse response as JSON (default: true)
 * @returns {Promise<{status: number, data: any, ok: boolean}>}
 */
async function apiCall(endpoint, options = {}) {
    const {
        method = 'GET',
        body = null,
        token = null,
        json = true
    } = options;

    const headers = {
        'Content-Type': 'application/json'
    };

    if (token) {
        headers['Authorization'] = `Bearer ${token}`;
    }

    const fetchOptions = {
        method,
        headers
    };

    if (body) {
        fetchOptions.body = JSON.stringify(body);
    }

    try {
        const res = await fetch(API_BASE + endpoint, fetchOptions);
        let data = null;

        if (json) {
            try {
                data = await res.json();
            } catch (err) {
                data = null;
            }
        } else {
            data = await res.text();
        }

        return {
            status: res.status,
            ok: res.ok,
            data
        };
    } catch (err) {
        return {
            status: 0,
            ok: false,
            data: null,
            error: err.message
        };
    }
}

/**
 * Show error message (requires showError function in global scope)
 */
function handleError(error, statusCode) {
    const errorMessages = {
        0: 'Network error: ' + error,
        400: 'Invalid request',
        401: 'Unauthorized - please log in',
        403: 'Access denied',
        404: 'Not found',
        409: 'Conflict - resource already exists',
        500: 'Server error'
    };

    const msg = errorMessages[statusCode] || errorMessages[0] || 'Error';
    if (typeof showError === 'function') {
        showError(msg);
    }
}

/**
 * Show success message (requires showSuccess function in global scope)
 */
function handleSuccess(message) {
    if (typeof showSuccess === 'function') {
        showSuccess(message);
    }
}
